use std::collections::{HashMap, VecDeque};
use std::convert::Infallible;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::Html;
use axum::routing::get;
use axum::{Json, Router};
use chrono::Utc;
use serde::Serialize;
use tokio::runtime::Builder;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::config::Config;
use crate::logging::write_aggregated_log_line;
use crate::services::{ServiceDashboard, ServiceEndpoint, SERVICES};

const EVENT_LIMIT: usize = 20;
const LOG_LIMIT: usize = 500;

struct ServiceCard {
    key: &'static str,
    dashboard: ServiceDashboard,
}

#[derive(Clone, Serialize)]
pub struct LogEntry {
    pub at: String,
    pub service: String,
    pub stream: String,
    pub line: String,
}

const SERVICE_ORDER: &[&str] = &[
    "sparql-proxy",
    "qlever",
    "virtuoso",
    "sparqlist",
    "grasp",
    "tabulae",
    "togomcp",
];

#[derive(Clone, Serialize)]
pub struct ExitInfo {
    pub detail: String,
    pub at: String,
}

#[derive(Clone, Serialize)]
pub struct ServiceStatusSnapshot {
    pub state: String,
    pub pid: Option<u32>,
    pub restart_count: u32,
    pub started_at: Option<String>,
    pub next_restart_at: Option<String>,
    pub last_exit: Option<ExitInfo>,
    pub message: String,
}

#[derive(Clone, Serialize)]
pub struct DashboardSnapshot {
    pub generated_at: String,
    pub services: HashMap<String, ServiceStatusSnapshot>,
    pub recent_events: Vec<String>,
    pub recent_logs: Vec<LogEntry>,
}

#[derive(Clone, Serialize)]
pub struct StatusSnapshot {
    pub generated_at: String,
    pub services: HashMap<String, ServiceStatusSnapshot>,
    pub recent_events: Vec<String>,
}

#[derive(Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum DashboardEvent {
    Snapshot { snapshot: DashboardSnapshot },
    Log { entry: LogEntry },
}

pub(crate) struct DashboardState {
    snapshot: DashboardSnapshot,
    updates: broadcast::Sender<String>,
}

pub type SharedDashboardState = Arc<Mutex<DashboardState>>;

#[derive(Clone)]
struct DashboardAppState {
    config: Config,
    state: SharedDashboardState,
}

static DASHBOARD_STATE: OnceLock<SharedDashboardState> = OnceLock::new();

pub fn initial_snapshot() -> DashboardState {
    let services = SERVICES
        .iter()
        .map(|spec| {
            (
                spec.name.to_string(),
                ServiceStatusSnapshot {
                    state: String::from("starting"),
                    pid: None,
                    restart_count: 0,
                    started_at: None,
                    next_restart_at: None,
                    last_exit: None,
                    message: String::from("Waiting for first start"),
                },
            )
        })
        .collect();

    let (updates, _) = broadcast::channel(64);

    DashboardState {
        snapshot: DashboardSnapshot {
            generated_at: now_rfc3339(),
            services,
            recent_events: Vec::new(),
            recent_logs: Vec::new(),
        },
        updates,
    }
}

pub fn now_rfc3339() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

pub fn spawn_dashboard_server(config: &Config, state: SharedDashboardState) -> Result<(), String> {
    let _ = DASHBOARD_STATE.set(state.clone());
    let address = format!("127.0.0.1:{}", config.supervisor_http_port);
    let listener = std::net::TcpListener::bind(&address).map_err(|error| {
        format!("dashboard event=listen-failed address={address} error={error}")
    })?;
    listener.set_nonblocking(true).map_err(|error| {
        format!("dashboard event=nonblocking-failed address={address} error={error}")
    })?;
    let config = config.clone();

    thread::spawn(move || {
        let runtime = match Builder::new_multi_thread().enable_all().build() {
            Ok(runtime) => runtime,
            Err(error) => {
                log_supervisor_message(&format!("dashboard event=runtime-failed error={error}"));
                return;
            }
        };

        runtime.block_on(async move {
            let listener = match tokio::net::TcpListener::from_std(listener) {
                Ok(listener) => listener,
                Err(error) => {
                    log_supervisor_message(&format!(
                        "dashboard event=listener-failed error={error}"
                    ));
                    return;
                }
            };

            let app = Router::new()
                .route("/", get(dashboard_page))
                .route("/logs", get(logs_page))
                .route("/api/status", get(status_api))
                .route("/api/events", get(events_stream))
                .with_state(DashboardAppState {
                    config,
                    state: state.clone(),
                });

            if let Err(error) = axum::serve(listener, app).await {
                log_supervisor_message(&format!("dashboard event=serve-failed error={error}"));
            }
        });
    });

    Ok(())
}

async fn dashboard_page(State(app): State<DashboardAppState>, headers: HeaderMap) -> Html<String> {
    let snapshot = current_snapshot(&app.config, &app.state);
    Html(render_html(&snapshot, &request_base_url(&headers)))
}

async fn logs_page(State(app): State<DashboardAppState>, headers: HeaderMap) -> Html<String> {
    let snapshot = current_snapshot(&app.config, &app.state);
    Html(render_logs_html(&snapshot, &request_base_url(&headers)))
}

async fn status_api(State(app): State<DashboardAppState>) -> Json<StatusSnapshot> {
    Json(status_snapshot(current_snapshot(&app.config, &app.state)))
}

async fn events_stream(
    State(app): State<DashboardAppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let (initial_messages, receiver) = match app.state.lock() {
        Ok(state) => (
            state
                .snapshot
                .recent_logs
                .iter()
                .filter_map(|entry| {
                    serialize_event(DashboardEvent::Log {
                        entry: entry.clone(),
                    })
                })
                .collect::<Vec<_>>(),
            state.updates.subscribe(),
        ),
        Err(_) => {
            let (fallback, _) = broadcast::channel(1);
            (Vec::new(), fallback.subscribe())
        }
    };
    let initial_stream = tokio_stream::iter(
        initial_messages
            .into_iter()
            .map(|message| Ok(Event::default().data(message))),
    );
    let update_stream = BroadcastStream::new(receiver).filter_map(|message| match message {
        Ok(message) => Some(Ok(Event::default().data(message))),
        Err(_) => None,
    });
    let stream = initial_stream.chain(update_stream);

    Sse::new(stream).keep_alive(KeepAlive::default())
}

pub fn log_supervisor_message(message: &str) {
    let line = format!("{} [supervisor] {}\n", now_rfc3339(), message);
    eprint!("{line}");
    let _ = write_aggregated_log_line(line.as_bytes());
    if let Some(state) = DASHBOARD_STATE.get() {
        record_log(state, "supervisor", "stderr", message);
    }
}

pub fn update_service(
    state: &SharedDashboardState,
    service_name: &str,
    update: impl FnOnce(&mut ServiceStatusSnapshot),
) {
    if let Ok(mut state) = state.lock() {
        if let Some(service) = state.snapshot.services.get_mut(service_name) {
            update(service);
        }
        state.snapshot.generated_at = now_rfc3339();
        broadcast_snapshot(&state);
    }
}

pub fn record_event(state: &SharedDashboardState, event: String) {
    if let Ok(mut state) = state.lock() {
        let mut recent = VecDeque::from(std::mem::take(&mut state.snapshot.recent_events));
        recent.push_back(format!("{} {}", now_rfc3339(), event));
        while recent.len() > EVENT_LIMIT {
            recent.pop_front();
        }
        state.snapshot.recent_events = recent.into_iter().collect();
        state.snapshot.generated_at = now_rfc3339();
        broadcast_snapshot(&state);
    }
}

pub fn record_log(state: &SharedDashboardState, service: &str, stream: &str, line: &str) {
    let line = line.trim_end_matches('\n').trim_end_matches('\r');
    if line.is_empty() {
        return;
    }

    if let Ok(mut state) = state.lock() {
        let entry = LogEntry {
            at: now_rfc3339(),
            service: service.to_string(),
            stream: stream.to_string(),
            line: line.to_string(),
        };
        let mut recent = VecDeque::from(std::mem::take(&mut state.snapshot.recent_logs));
        recent.push_back(entry.clone());
        while recent.len() > LOG_LIMIT {
            recent.pop_front();
        }
        state.snapshot.recent_logs = recent.into_iter().collect();
        state.snapshot.generated_at = now_rfc3339();
        broadcast_event(&state.updates, DashboardEvent::Log { entry });
    }
}

fn broadcast_snapshot(state: &DashboardState) {
    broadcast_event(
        &state.updates,
        DashboardEvent::Snapshot {
            snapshot: state.snapshot.clone(),
        },
    );
}

fn broadcast_event(updates: &broadcast::Sender<String>, event: DashboardEvent) {
    if let Some(message) = serialize_event(event) {
        let _ = updates.send(message);
    }
}

fn serialize_event(event: DashboardEvent) -> Option<String> {
    serde_json::to_string(&event).ok()
}

fn current_snapshot(_config: &Config, state: &SharedDashboardState) -> DashboardSnapshot {
    match state.lock() {
        Ok(state) => enrich_snapshot(state.snapshot.clone()),
        Err(_) => enrich_snapshot(initial_snapshot().snapshot),
    }
}

fn status_snapshot(snapshot: DashboardSnapshot) -> StatusSnapshot {
    StatusSnapshot {
        generated_at: snapshot.generated_at,
        services: snapshot.services,
        recent_events: snapshot.recent_events,
    }
}

fn render_html(snapshot: &DashboardSnapshot, base_url: &str) -> String {
    let service_cards = dashboard_cards()
        .iter()
        .map(|card| render_service_card(snapshot, card, base_url))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>TogoPackage Supervisor</title>
  <style>
    :root {{
      color-scheme: light;
      --base-100: #ffffff;
      --base-200: #f3f4f6;
      --base-300: #e5e7eb;
      --base-content: #111827;
      --muted: #6b7280;
      --success: #16a34a;
      --warning: #d97706;
      --error: #dc2626;
    }}
    * {{ box-sizing: border-box; }}
    html, body {{ height: 100%; }}
    body {{
      margin: 0;
      font-family: ui-sans-serif, system-ui, sans-serif;
      background: var(--base-200);
      color: var(--base-content);
      overflow-y: auto;
    }}
    a {{
      color: inherit;
      text-decoration: none;
    }}
    .navbar {{
      background: var(--base-100);
      box-shadow: 0 1px 2px rgba(17, 24, 39, 0.08);
      padding: 0 1rem;
    }}
    .navbar-inner {{
      max-width: 48rem;
      margin: 0 auto;
      min-height: 4rem;
      display: flex;
      align-items: center;
    }}
    .btn-ghost {{
      display: inline-flex;
      align-items: center;
      min-height: 2.5rem;
      padding: 0 0.75rem;
      border-radius: 0.5rem;
      font-size: 1.25rem;
      font-weight: 600;
    }}
    .container {{
      width: 100%;
      max-width: 48rem;
      margin: 0 auto;
      padding: 2.5rem 1rem;
    }}
    .grid {{
      display: grid;
      gap: 1.4rem;
    }}
    .card {{
      display: grid;
      gap: 0.6rem;
    }}
    .card-link {{
      display: block;
      background: var(--base-100);
      border: 1px solid var(--base-300);
      border-radius: 1rem;
      transition: transform 140ms ease, box-shadow 140ms ease, border-color 140ms ease, background-color 140ms ease;
    }}
    .card-link:hover,
    .card-link:focus-visible {{
      transform: translateY(-2px);
      box-shadow: 0 16px 30px rgba(17, 24, 39, 0.12);
      background: #fcfcfd;
      border-color: #cbd5e1;
      outline: none;
    }}
    .card-static {{
      display: block;
      background: var(--base-100);
      border: 1px solid var(--base-300);
      border-radius: 1rem;
    }}
    .card-body {{
      padding: 1rem;
    }}
    .card-heading {{
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 0.75rem;
    }}
    .card-title {{
      margin: 0;
      font-size: 1.125rem;
      font-weight: 700;
    }}
    .card-title-row {{
      display: flex;
      align-items: center;
      gap: 0.6rem;
      min-width: 0;
    }}
    .card-title-text {{
      min-width: 0;
    }}
    .card-link-indicator {{
      flex-shrink: 0;
      display: inline-flex;
      align-items: center;
      justify-content: center;
      width: 1.9rem;
      height: 1.9rem;
      border-radius: 9999px;
      background: var(--base-200);
      color: var(--muted);
      font-size: 1rem;
      font-weight: 700;
      transition: transform 140ms ease, background-color 140ms ease, color 140ms ease;
    }}
    .card-link:hover .card-link-indicator,
    .card-link:focus-visible .card-link-indicator {{
      transform: translateX(2px);
      background: #dbeafe;
      color: #1d4ed8;
    }}
    .link-hover:hover {{
      text-decoration: underline;
    }}
    p {{
      margin: 0.5rem 0 0;
      color: var(--muted);
    }}
    .badge {{
      display: inline-flex;
      align-items: center;
      border-radius: 9999px;
      padding: 0.2rem 0.65rem;
      font-size: 0.8rem;
      font-weight: 700;
      border: 1px solid currentColor;
      white-space: nowrap;
    }}
    .badge-running {{ color: var(--success); }}
    .badge-starting, .badge-restarting, .badge-stopping {{ color: var(--warning); }}
    .badge-failed, .badge-stopped {{ color: var(--error); }}
    .badge-soft {{
      background: color-mix(in srgb, currentColor 12%, white);
    }}
    .card-meta {{
      display: grid;
      gap: 0.45rem;
      margin-left: 1rem;
      padding-left: 0.9rem;
      border-left: 2px solid var(--base-300);
    }}
    .detail-panel, .endpoints {{
      border-radius: 0.75rem;
      background: var(--base-200);
      padding: 0.75rem;
    }}
    .details-toggle {{
      display: grid;
      gap: 0.55rem;
    }}
    .details-toggle > summary {{
      list-style: none;
      display: inline-flex;
      align-items: center;
      justify-content: center;
      width: fit-content;
      min-height: 2.2rem;
      padding: 0.45rem 0.8rem;
      border-radius: 9999px;
      border: 1px solid var(--base-300);
      background: var(--base-100);
      cursor: pointer;
      font-size: 0.85rem;
      font-weight: 700;
      color: var(--base-content);
      transition: border-color 140ms ease, background-color 140ms ease;
    }}
    .details-toggle > summary::-webkit-details-marker {{
      display: none;
    }}
    .details-toggle > summary:hover,
    .details-toggle > summary:focus-visible {{
      border-color: #93c5fd;
      background: #eff6ff;
    }}
    .details-toggle > summary::before {{
      content: "Show details";
    }}
    .details-toggle[open] > summary::before {{
      content: "Hide details";
    }}
    .detail-grid {{
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 0.75rem 1rem;
    }}
    .detail-label {{
      font-size: 0.75rem;
      font-weight: 600;
      color: var(--muted);
    }}
    .detail-value {{
      margin-top: 0.15rem;
      word-break: break-word;
    }}
    .endpoint-list {{
      display: grid;
      gap: 0.75rem;
    }}
    .endpoint-item + .endpoint-item {{
      margin-top: 0;
    }}
    .endpoint-label {{
      font-size: 0.875rem;
      font-weight: 600;
      margin-bottom: 0.35rem;
    }}
    .endpoint-row {{
      display: flex;
      align-items: center;
      gap: 0.55rem;
    }}
    .endpoint-url {{
      flex: 1;
      border: 1px solid var(--base-300);
      border-radius: 0.5rem;
      background: var(--base-100);
      padding: 0.55rem 0.75rem;
      overflow: auto;
      white-space: nowrap;
      user-select: text;
    }}
    .endpoint-copy {{
      flex-shrink: 0;
      min-height: 2.5rem;
      padding: 0.55rem 0.8rem;
      border: 1px solid var(--base-300);
      border-radius: 0.5rem;
      background: var(--base-100);
      color: var(--base-content);
      font: inherit;
      font-size: 0.85rem;
      font-weight: 700;
      cursor: pointer;
      transition: border-color 140ms ease, background-color 140ms ease;
    }}
    .endpoint-copy:hover,
    .endpoint-copy:focus-visible {{
      border-color: #93c5fd;
      background: #eff6ff;
    }}
    code {{
      font-family: ui-monospace, SFMono-Regular, monospace;
    }}
    @media (max-width: 640px) {{
      .endpoint-row {{
        flex-direction: column;
        align-items: stretch;
      }}
      .endpoint-copy {{
        width: 100%;
      }}
      .detail-grid {{
        grid-template-columns: 1fr;
      }}
    }}
  </style>
</head>
<body>
  <div class="navbar">
    <div class="navbar-inner">
      <a href="/" class="btn-ghost">TogoPackage</a>
      <a href="/logs" class="link-hover">Logs</a>
    </div>
  </div>
  <main class="container">
    <section class="grid">{}</section>
  </main>
  <script>
    document.querySelectorAll("[data-copy-target]").forEach((button) => {{
      button.addEventListener("click", async () => {{
        const target = document.getElementById(button.dataset.copyTarget);
        if (!target) return;

        try {{
          await navigator.clipboard.writeText(target.textContent ?? "");
          button.textContent = "Copied";
          window.setTimeout(() => {{
            button.textContent = button.dataset.defaultLabel ?? "Copy";
          }}, 1200);
        }} catch (_error) {{
          const selection = window.getSelection();
          const range = document.createRange();
          range.selectNodeContents(target);
          selection?.removeAllRanges();
          selection?.addRange(range);
        }}
      }});

      button.dataset.defaultLabel = button.textContent;
    }});
  </script>
</body>
</html>"#,
        service_cards
    )
}

fn render_service_card(snapshot: &DashboardSnapshot, card: &ServiceCard, base_url: &str) -> String {
    let service = snapshot.services.get(card.key);
    let state = service
        .map(|service| service.state.as_str())
        .unwrap_or("starting");
    let title = match card.dashboard.href {
        Some(_) => format!(
            r#"<div class="card-title-row"><h2 class="card-title card-title-text">{}</h2><span class="card-link-indicator" aria-hidden="true">→</span></div>"#,
            escape_html(card.dashboard.title)
        ),
        None => format!(
            r#"<div class="card-title-row"><h2 class="card-title card-title-text">{}</h2></div>"#,
            escape_html(card.dashboard.title)
        ),
    };
    let card_body = format!(
        r#"<div class="card-body">
    <div class="card-heading">
      {}
      <span class="badge badge-soft badge-{}">{}</span>
    </div>
    <p>{}</p>
  </div>"#,
        title,
        state_class(state),
        escape_html(state_label(state)),
        escape_html(card.dashboard.description),
    );

    let main = match card.dashboard.href {
        Some(href) => format!(
            r#"<a href="{href}" class="card-link" aria-label="Open {}">{}</a>"#,
            escape_html(card.dashboard.title),
            card_body,
        ),
        None => format!(r#"<div class="card-static">{}</div>"#, card_body),
    };

    let meta = render_card_meta(card.dashboard.endpoints, service, base_url);

    format!(r#"<section class="card">{}{}</section>"#, main, meta)
}

fn render_card_meta(
    endpoints: &[ServiceEndpoint],
    service: Option<&ServiceStatusSnapshot>,
    base_url: &str,
) -> String {
    let details = render_details(service);
    let endpoints = render_endpoints(endpoints, base_url);

    if details.is_empty() && endpoints.is_empty() {
        String::new()
    } else {
        format!(r#"<div class="card-meta">{}{}</div>"#, details, endpoints)
    }
}

fn render_details(service: Option<&ServiceStatusSnapshot>) -> String {
    let Some(service) = service else {
        return String::from(
            r#"<details class="details-toggle"><summary></summary><div class="detail-panel"><div class="detail-grid"><div><div class="detail-label">Status</div><div class="detail-value">Waiting for first update</div></div></div></div></details>"#,
        );
    };

    let last_exit = service
        .last_exit
        .as_ref()
        .map(|exit| format!("{} at {}", escape_html(&exit.detail), escape_html(&exit.at)))
        .unwrap_or_else(|| String::from("-"));

    format!(
        r#"<details class="details-toggle">
  <summary></summary>
  <div class="detail-panel">
  <div class="detail-grid">
    <div><div class="detail-label">Message</div><div class="detail-value">{}</div></div>
    <div><div class="detail-label">PID</div><div class="detail-value">{}</div></div>
    <div><div class="detail-label">Restarts</div><div class="detail-value">{}</div></div>
    <div><div class="detail-label">Started</div><div class="detail-value">{}</div></div>
    <div><div class="detail-label">Next restart</div><div class="detail-value">{}</div></div>
    <div><div class="detail-label">Last exit</div><div class="detail-value">{}</div></div>
  </div>
  </div>
</details>"#,
        escape_html(&service.message),
        service
            .pid
            .map(|pid| pid.to_string())
            .unwrap_or_else(|| String::from("-")),
        service.restart_count,
        escape_html(service.started_at.as_deref().unwrap_or("-")),
        escape_html(service.next_restart_at.as_deref().unwrap_or("-")),
        last_exit,
    )
}

fn render_endpoints(endpoints: &[ServiceEndpoint], base_url: &str) -> String {
    if endpoints.is_empty() {
        return String::new();
    }

    let items = endpoints
        .iter()
        .enumerate()
        .map(|(index, endpoint)| {
            let url = absolute_url(base_url, endpoint.path);
            format!(
                r#"<div class="endpoint-item">
  <div class="endpoint-label">{}</div>
  <div class="endpoint-row">
    <div class="endpoint-url"><code id="endpoint-{}">{}</code></div>
    <button type="button" class="endpoint-copy" data-copy-target="endpoint-{}">{}</button>
  </div>
</div>"#,
                escape_html(endpoint.label),
                index,
                escape_html(&url),
                index,
                "Copy",
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(r#"<div class="endpoints"><div class="endpoint-list">{items}</div></div>"#)
}

fn dashboard_cards() -> Vec<ServiceCard> {
    let mut cards = SERVICES
        .iter()
        .filter(|spec| spec.dashboard.show)
        .map(|spec| ServiceCard {
            key: spec.name,
            dashboard: spec.dashboard,
        })
        .collect::<Vec<_>>();
    cards.sort_by_key(|card| {
        SERVICE_ORDER
            .iter()
            .position(|name| name == &card.key)
            .unwrap_or(SERVICE_ORDER.len())
    });
    cards
}

fn enrich_snapshot(mut snapshot: DashboardSnapshot) -> DashboardSnapshot {
    snapshot.generated_at = now_rfc3339();
    snapshot
}

fn render_logs_html(_: &DashboardSnapshot, _: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>TogoPackage Logs</title>
  <style>
    :root {{
      color-scheme: light;
      --base-100: #ffffff;
      --base-200: #f3f4f6;
      --base-300: #e5e7eb;
      --base-content: #111827;
      --muted: #6b7280;
    }}
    * {{ box-sizing: border-box; }}
    html, body {{ height: 100%; }}
    body {{
      margin: 0;
      font-family: ui-sans-serif, system-ui, sans-serif;
      background: var(--base-200);
      color: var(--base-content);
      height: 100vh;
      overflow: hidden;
      display: flex;
      flex-direction: column;
    }}
    a {{ color: inherit; text-decoration: none; }}
    .navbar {{
      background: var(--base-100);
      box-shadow: 0 1px 2px rgba(17, 24, 39, 0.08);
      padding: 0 1rem;
    }}
    .navbar-inner {{
      max-width: 48rem;
      margin: 0 auto;
      min-height: 4rem;
      display: flex;
      align-items: center;
    }}
    .nav-links {{
      display: flex;
      gap: 0.75rem;
      align-items: center;
    }}
    .btn-ghost {{
      display: inline-flex;
      align-items: center;
      min-height: 2.5rem;
      padding: 0 0.75rem;
      border-radius: 0.5rem;
      font-size: 1.25rem;
      font-weight: 600;
    }}
    .link-hover:hover {{ text-decoration: underline; }}
    .container {{
      flex: 1;
      width: 100%;
      max-width: none;
      margin: 0;
      padding: 0;
      min-height: 0;
      overflow: hidden;
    }}
    p {{ margin: 0.5rem 0 0; color: var(--muted); }}
    .log-list {{
      display: grid;
      gap: 0;
      background: #111827;
      height: 100%;
      overflow-y: auto;
      overflow-x: hidden;
    }}
    .log-line {{
      display: grid;
      grid-template-columns: 12rem 8rem 5rem 1fr;
      gap: 0.75rem;
      padding: 0.3rem 0.75rem;
      background: #111827;
      color: #e5e7eb;
      align-items: start;
      line-height: 1.2;
    }}
    .log-time, .log-service, .log-stream {{
      color: #93c5fd;
      font-family: ui-monospace, SFMono-Regular, monospace;
      font-size: 0.76rem;
      white-space: nowrap;
      line-height: 1.2;
    }}
    code {{
      font-family: ui-monospace, SFMono-Regular, monospace;
      white-space: pre-wrap;
      word-break: break-word;
      font-size: 0.8rem;
      line-height: 1.2;
    }}
    @media (max-width: 900px) {{
      .log-line {{
        grid-template-columns: 1fr;
      }}
      .log-time, .log-service, .log-stream {{
        white-space: normal;
      }}
    }}
  </style>
</head>
<body>
  <div class="navbar">
    <div class="navbar-inner">
      <a href="/" class="btn-ghost">TogoPackage</a>
      <div class="nav-links">
        <a href="/" class="link-hover">Dashboard</a>
      </div>
    </div>
  </div>
  <main class="container">
    <div class="log-list" id="log-list"></div>
  </main>
  <script>
    const logList = document.getElementById("log-list");
    const renderLogLine = (entry) => {{
      const line = document.createElement("div");
      line.className = "log-line";

      const time = document.createElement("span");
      time.className = "log-time";
      time.textContent = entry.at;

      const service = document.createElement("span");
      service.className = "log-service";
      service.textContent = entry.service;

      const stream = document.createElement("span");
      stream.className = "log-stream";
      stream.textContent = entry.stream;

      const code = document.createElement("code");
      code.textContent = entry.line;

      line.append(time, service, stream, code);
      return line;
    }};

    const scrollToBottom = () => {{
      logList.scrollTop = logList.scrollHeight;
    }};

    if (logList) {{
      scrollToBottom();
      window.addEventListener("load", () => {{
        const eventSource = new EventSource("/api/events");
        eventSource.onmessage = (event) => {{
          let message;
          try {{
            message = JSON.parse(event.data);
          }} catch {{
            return;
          }}

          if (message.type !== "log" || !message.entry) {{
            return;
          }}

          const distanceFromBottom = logList.scrollHeight - logList.scrollTop - logList.clientHeight;
          const shouldStickToBottom = distanceFromBottom < 24;
          logList.appendChild(renderLogLine(message.entry));

          while (logList.childElementCount > {LOG_LIMIT}) {{
            logList.firstElementChild?.remove();
          }}

          if (shouldStickToBottom) {{
            scrollToBottom();
          }}
        }};
        window.addEventListener("beforeunload", () => {{
          eventSource.close();
        }}, {{ once: true }});
      }}, {{ once: true }});
    }}
  </script>
</body>
</html>"#,
    )
}

fn state_label(state: &str) -> &str {
    match state {
        "running" => "Running",
        "completed" => "Completed",
        "failed" => "Failed",
        "stopped" => "Stopped",
        "restarting" => "Restarting",
        "stopping" => "Stopping",
        _ => "Starting",
    }
}

fn state_class(state: &str) -> &str {
    match state {
        "running" | "completed" => "running",
        "failed" | "stopped" => "failed",
        "restarting" => "restarting",
        "stopping" => "stopping",
        _ => "starting",
    }
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn request_base_url(headers: &HeaderMap) -> String {
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .unwrap_or("http");
    let host = headers
        .get("host")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .unwrap_or("localhost:7000");
    format!("{scheme}://{host}")
}

fn absolute_url(base_url: &str, path: &str) -> String {
    if path.starts_with("http://") || path.starts_with("https://") {
        return path.to_string();
    }

    if path.starts_with('/') {
        return format!("{base_url}{path}");
    }

    format!("{base_url}/{path}")
}
