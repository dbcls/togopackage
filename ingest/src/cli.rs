use clap::{Args, Parser};
use std::path::PathBuf;

use crate::model::{RuntimePaths, VirtuosoTuning};

#[derive(Debug, Parser)]
#[command(name = "togopackage-ingest")]
pub struct Cli {
    #[command(flatten)]
    pub args: CommonArgs,
}

#[derive(Debug, Args, Clone)]
pub struct CommonArgs {
    #[arg(long, env = "TOGOPACKAGE_CONFIG", default_value = "/data/config.yaml")]
    pub config_path: PathBuf,
    #[arg(long, env = "QLEVER_DATA_DIR", default_value = "/data/sources")]
    pub qlever_data_dir: PathBuf,
    #[arg(long, env = "SOURCE_MANIFEST_PATH")]
    pub source_manifest_path: Option<PathBuf>,
    #[arg(
        long,
        env = "QLEVER_INDEX_BASE",
        default_value = "/data/qlever/index/default"
    )]
    pub qlever_index_base: String,
    #[arg(long, env = "VIRTUOSO_DATA_DIR", default_value = "/data/virtuoso")]
    pub virtuoso_data_dir: PathBuf,
    #[arg(long, env = "VIRTUOSO_INI_PATH")]
    pub virtuoso_ini_path: Option<PathBuf>,
    #[arg(long, env = "VIRTUOSO_HTTP_PORT", default_value = "8890")]
    pub virtuoso_http_port: String,
    #[arg(long, env = "VIRTUOSO_ISQL_PORT", default_value = "1111")]
    pub virtuoso_isql_port: String,
    #[arg(long, env = "VIRTUOSO_DBA_PASSWORD", default_value = "dba")]
    pub virtuoso_dba_password: String,
    #[arg(long, env = "VIRTUOSO_NUMBER_OF_BUFFERS", default_value = "170000")]
    pub virtuoso_number_of_buffers: String,
    #[arg(long, env = "VIRTUOSO_MAX_DIRTY_BUFFERS", default_value = "130000")]
    pub virtuoso_max_dirty_buffers: String,
    #[arg(long, env = "VIRTUOSO_MAX_CHECKPOINT_REMAP", default_value = "500")]
    pub virtuoso_max_checkpoint_remap: String,
    #[arg(long, env = "VIRTUOSO_CHECKPOINT_INTERVAL", default_value = "120")]
    pub virtuoso_checkpoint_interval: String,
    #[arg(long, env = "VIRTUOSO_MAX_QUERY_MEM", default_value = "512M")]
    pub virtuoso_max_query_mem: String,
    #[arg(long, env = "VIRTUOSO_SERVER_THREADS", default_value = "4")]
    pub virtuoso_server_threads: String,
    #[arg(long, env = "VIRTUOSO_MAX_CLIENT_CONNECTIONS", default_value = "8")]
    pub virtuoso_max_client_connections: String,
}

impl From<CommonArgs> for RuntimePaths {
    fn from(args: CommonArgs) -> Self {
        let source_manifest_path = args
            .source_manifest_path
            .unwrap_or_else(|| args.qlever_data_dir.join("source-manifest.json"));
        let virtuoso_ini_path = args
            .virtuoso_ini_path
            .unwrap_or_else(|| args.virtuoso_data_dir.join("virtuoso.ini"));

        Self {
            config_path: args.config_path,
            qlever_data_dir: args.qlever_data_dir,
            source_manifest_path,
            qlever_index_base: args.qlever_index_base,
            virtuoso_data_dir: args.virtuoso_data_dir,
            virtuoso_ini_path,
            virtuoso_http_port: args.virtuoso_http_port,
            virtuoso_isql_port: args.virtuoso_isql_port,
            virtuoso_dba_password: args.virtuoso_dba_password,
            virtuoso_tuning: VirtuosoTuning {
                number_of_buffers: args.virtuoso_number_of_buffers,
                max_dirty_buffers: args.virtuoso_max_dirty_buffers,
                max_checkpoint_remap: args.virtuoso_max_checkpoint_remap,
                checkpoint_interval: args.virtuoso_checkpoint_interval,
                max_query_mem: args.virtuoso_max_query_mem,
                server_threads: args.virtuoso_server_threads,
                max_client_connections: args.virtuoso_max_client_connections,
            },
        }
    }
}
