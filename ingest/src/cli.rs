use clap::{Args, Parser};
use std::path::PathBuf;

use crate::model::RuntimePaths;

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
        }
    }
}
