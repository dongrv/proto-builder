use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "proto-builder",
    version,
    about = "一站式 protobuf → Go 开发工具 / Self-contained protobuf → Go builder",
    subcommand_required = false
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Cmd>,
    #[command(flatten)]
    pub options: Options,
}
#[derive(Debug, Clone, Default, Args)]
pub struct Options {
    #[arg(
        long,
        global = true,
        help = "Agent 稳定 JSON 摘要与非交互调用 / Versioned agent interface"
    )]
    pub agent: bool,
    #[arg(long,global=true,num_args=0..=1,default_missing_value="true")]
    pub docs: Option<bool>,
    #[arg(long,global=true,num_args=0..=1,default_missing_value="true")]
    pub protocol_config: Option<bool>,
    #[arg(long,global=true,num_args=0..=1,default_missing_value="true")]
    pub copy_to_calc: Option<bool>,
    #[arg(long,global=true,num_args=0..=1,default_missing_value="true")]
    pub svn_update: Option<bool>,
    #[arg(
        long,
        global = true,
        value_name = "FILE",
        help = "指定 TOML 配置 / Explicit configuration"
    )]
    pub config: Option<PathBuf>,
    #[arg(long, global = true, help = "协议项目根目录 / Protocol project root")]
    pub repo_root: Option<PathBuf>,
    #[arg(long, alias = "proto_path", global = true)]
    pub proto_path: Option<PathBuf>,
    #[arg(long, alias = "server_path", global = true)]
    pub server_path: Option<PathBuf>,
    #[arg(long, alias = "calc_path", global = true)]
    pub calc_path: Option<PathBuf>,
    #[arg(long, global = true, help = "开发者名字 / Developer name")]
    pub developer_name: Option<String>,
    #[arg(
        long,
        alias = "assignID",
        global = true,
        help = "兼容旧 ID 选择 / Legacy activity or machine ID"
    )]
    pub assign_id: Option<u64>,
    #[arg(
        long = "activity-id",
        global = true,
        value_delimiter = ',',
        help = "活动 ID（可多选） / Activity IDs"
    )]
    pub activity_ids: Vec<String>,
    #[arg(
        long = "machine-id",
        global = true,
        value_delimiter = ',',
        help = "机器 ID（可多选） / Machine IDs"
    )]
    pub machine_ids: Vec<String>,
    #[arg(
        long = "exclude",
        global = true,
        value_delimiter = ',',
        help = "排除路径、目录名或 glob / Exclude paths or globs"
    )]
    pub excludes: Vec<String>,
    #[arg(
        long,
        global = true,
        help = "仅预览，不更新 SVN 或生成文件 / Read-only plan"
    )]
    pub dry_run: bool,
    #[arg(long, global = true)]
    pub no_docs: bool,
    #[arg(long, global = true)]
    pub no_config: bool,
    #[arg(long, global = true)]
    pub no_copy: bool,
    #[arg(
        long,
        global = true,
        help = "禁用编译前 SVN 更新 / Skip pre-build SVN update"
    )]
    pub no_svn_update: bool,
    #[arg(
        long,
        global = true,
        help = "显式覆盖已登记且被修改的生成物，并保留备份 / Replace modified owned outputs"
    )]
    pub force: bool,
    #[arg(
        long,
        global = true,
        help = "首次迁移时显式接管已有生成物并备份 / Adopt existing generated files with backups"
    )]
    pub adopt_existing: bool,
    #[arg(long, global = true, value_name = "N")]
    pub jobs: Option<usize>,
    #[arg(
        long,
        global = true,
        value_name = "SECONDS",
        help = "单条命令超时 / Per-command timeout"
    )]
    pub timeout_seconds: Option<u64>,
    #[arg(long,global=true,value_parser=["zh-CN","en-US"])]
    pub lang: Option<String>,
    #[arg(
        long,
        global = true,
        help = "输出单个 JSON 报告 / Machine-readable report"
    )]
    pub json: bool,
    #[arg(long, global = true, help = "禁止交互 / Never prompt")]
    pub non_interactive: bool,
    #[arg(long, global = true, help = "打开键盘向导 / Open keyboard wizard")]
    pub interactive: bool,
    #[arg(long, global = true)]
    pub report_json: Option<PathBuf>,
    #[arg(short, long, global = true)]
    pub verbose: bool,
}
#[derive(Debug, Subcommand)]
pub enum Cmd {
    #[command(about = "编译并安全写回 / Generate and deploy")]
    Build,
    #[command(about = "打开本地网页工作台 / Open the local web interface")]
    Web {
        #[arg(
            long,
            default_value_t = 0,
            help = "本地端口（0 自动分配） / Local port (0 = automatic)"
        )]
        port: u16,
        #[arg(long, help = "只启动服务，不自动打开浏览器 / Do not open a browser")]
        no_open: bool,
    },
    #[command(about = "只读扫描、筛选与输出计划 / Read-only plan")]
    Plan,
    #[command(about = "检查配置、工具链、SVN 和输出权限 / Diagnose environment")]
    Doctor,
    #[command(about = "导出内置 Codex Skill（不覆盖已有文件） / Export bundled Codex skill")]
    Skill {
        #[arg(long, value_name = "DIRECTORY")]
        output: PathBuf,
    },
    #[command(about = "与内部基准逐字节比较 / Compare internal baseline")]
    Compat {
        #[arg(long)]
        baseline: PathBuf,
        #[arg(long)]
        actual: PathBuf,
    },
    #[command(
        subcommand,
        about = "查看、校验、初始化或迁移配置 / Manage configuration"
    )]
    Config(ConfigCmd),
    #[command(about = "安装当前二进制并注册 PATH / Install current executable")]
    Install {
        #[arg(long)]
        system: bool,
        #[arg(long)]
        dir: Option<PathBuf>,
    },
    #[command(about = "卸载程序 / Uninstall")]
    Uninstall {
        #[arg(long)]
        system: bool,
        #[arg(long)]
        purge: bool,
    },
    #[command(subcommand, about = "检查或应用官方签名更新 / Signed updates")]
    SelfUpdate(UpdateCmd),
    #[command(about = "版本、平台与工具链 / Version and toolchain")]
    Version,
    #[command(about = "内置第三方工具与依赖许可 / Third-party licenses")]
    Licenses,
}
#[derive(Debug, Subcommand)]
pub enum ConfigCmd {
    Show {
        #[arg(
            long,
            help = "解释配置来源与覆盖关系 / Explain configuration provenance"
        )]
        explain: bool,
    },
    Check,
    Init {
        #[arg(long)]
        output: Option<PathBuf>,
    },
    Migrate {
        #[arg(long)]
        check: bool,
    },
}
#[derive(Debug, Subcommand)]
pub enum UpdateCmd {
    Check,
    Apply {
        #[arg(long,value_parser=["stable","beta"])]
        channel: Option<String>,
    },
    Rollback,
}
impl Cmd {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Build => "build",
            Self::Web { .. } => "web",
            Self::Plan => "plan",
            Self::Doctor => "doctor",
            Self::Skill { .. } => "skill",
            Self::Compat { .. } => "compat",
            Self::Config(_) => "config",
            Self::Install { .. } => "install",
            Self::Uninstall { .. } => "uninstall",
            Self::SelfUpdate(_) => "self-update",
            Self::Version => "version",
            Self::Licenses => "licenses",
        }
    }
}
