use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "proto-builder",
    version,
    about = "一站式 protobuf → Go 开发工具 / Self-contained protobuf → Go builder",
    subcommand_required = false,
    after_help = "常用入口 / Examples:\n  proto-builder                       # 键盘向导 / keyboard wizard\n  proto-builder web                   # 网页工作台 / browser\n  proto-builder plan --activity-id 7001,7002\n  proto-builder build --no-protocol-config\n  proto-builder agent capabilities\n\n路径默认从团队环境、项目配置或上次成功设置读取；相对路径基于协议项目根目录。\nPaths use team environment, project configuration or remembered setup; relative paths start at the protocol project root."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Cmd>,
    #[command(flatten)]
    pub options: Options,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Args)]
pub struct Options {
    #[arg(long,global=true,num_args=0..=1,default_missing_value="true", help = "生成完整共享 HTML/Markdown，默认开启 / Generate full shared documentation (default: on)", help_heading = "生成与覆盖 / Generation")]
    pub docs: Option<bool>,
    #[arg(long,global=true,num_args=0..=1,default_missing_value="true", help = "生成协议号配置，默认开启 / Generate protocol ID configuration (default: on)", help_heading = "生成与覆盖 / Generation")]
    pub protocol_config: Option<bool>,
    #[arg(long,global=true,num_args=0..=1,default_missing_value="true", help = "同步计算服，默认开启 / Sync calculation server (default: on)", help_heading = "生成与覆盖 / Generation")]
    pub copy_to_calc: Option<bool>,
    #[arg(long,global=true,num_args=0..=1,default_missing_value="true", help = "编译前更新 SVN，默认开启；plan 不更新 / Update SVN before builds (default: on; never in plan)", help_heading = "执行策略 / Execution")]
    pub svn_update: Option<bool>,
    #[arg(
        long,
        global = true,
        value_name = "FILE",
        help = "指定 TOML 配置 / Explicit configuration",
        help_heading = "项目与路径 / Project paths"
    )]
    pub config: Option<PathBuf>,
    #[arg(
        long,
        global = true,
        help = "协议项目根目录 / Protocol project root",
        help_heading = "项目与路径 / Project paths"
    )]
    pub repo_root: Option<PathBuf>,
    #[arg(
        long,
        alias = "proto_path",
        global = true,
        help = "Proto 源目录，默认 proto，相对协议项目根目录 / Proto sources (default: proto, relative to project root)",
        help_heading = "项目与路径 / Project paths"
    )]
    pub proto_path: Option<PathBuf>,
    #[arg(
        long,
        alias = "server_path",
        global = true,
        help = "业务服 Go 工程目录，输出到其 protocol/configs / Server Go project; writes protocol/configs beneath it",
        help_heading = "项目与路径 / Project paths"
    )]
    pub server_path: Option<PathBuf>,
    #[arg(
        long,
        alias = "calc_path",
        global = true,
        help = "计算服 Go 工程目录，输出到其 protocol / Calculation Go project; writes protocol beneath it",
        help_heading = "项目与路径 / Project paths"
    )]
    pub calc_path: Option<PathBuf>,
    #[arg(
        long,
        global = true,
        help = "开发者名字 / Developer name",
        help_heading = "项目与路径 / Project paths"
    )]
    pub developer_name: Option<String>,
    #[arg(
        long,
        alias = "assignID",
        global = true,
        help = "兼容旧 ID 选择 / Legacy activity or machine ID",
        help_heading = "协议筛选 / Selection"
    )]
    pub assign_id: Option<u64>,
    #[arg(
        long = "activity-id",
        global = true,
        value_delimiter = ',',
        help = "活动 ID（可多选） / Activity IDs",
        help_heading = "协议筛选 / Selection"
    )]
    pub activity_ids: Vec<String>,
    #[arg(
        long = "machine-id",
        global = true,
        value_delimiter = ',',
        help = "机器 ID（可多选） / Machine IDs",
        help_heading = "协议筛选 / Selection"
    )]
    pub machine_ids: Vec<String>,
    #[arg(
        long = "exclude",
        global = true,
        value_delimiter = ',',
        help = "排除路径、目录名或 glob / Exclude paths or globs",
        help_heading = "协议筛选 / Selection"
    )]
    pub excludes: Vec<String>,
    #[arg(
        long,
        global = true,
        help = "仅预览，不更新 SVN 或生成文件 / Read-only plan",
        help_heading = "执行策略 / Execution"
    )]
    pub dry_run: bool,
    #[arg(
        long,
        global = true,
        help = "关闭共享协议文档生成 / Skip documentation generation",
        help_heading = "生成与覆盖 / Generation"
    )]
    pub no_docs: bool,
    #[arg(
        long = "no-protocol-config",
        visible_alias = "no-config",
        global = true,
        help = "关闭协议号配置生成；仍读取 TOML 配置 / Skip protocol ID generation; still read TOML configuration",
        help_heading = "生成与覆盖 / Generation"
    )]
    pub no_config: bool,
    #[arg(
        long,
        global = true,
        help = "关闭计算服同步 / Skip calculation server synchronization",
        help_heading = "生成与覆盖 / Generation"
    )]
    pub no_copy: bool,
    #[arg(
        long,
        global = true,
        help = "禁用编译前 SVN 更新 / Skip pre-build SVN update",
        help_heading = "执行策略 / Execution"
    )]
    pub no_svn_update: bool,
    #[arg(
        long,
        global = true,
        help = "备份后覆盖已修改的其他生成物或清理已修改的过期产物；pb.go 默认覆盖 / Replace other modified outputs with backups",
        help_heading = "生成与覆盖 / Generation"
    )]
    pub force: bool,
    #[arg(
        long,
        global = true,
        help = "备份后接管其他已有生成物；pb.go 默认接管并覆盖 / Adopt other existing outputs with backups",
        help_heading = "生成与覆盖 / Generation"
    )]
    pub adopt_existing: bool,
    #[arg(
        long,
        global = true,
        value_name = "N",
        help = "并行任务数，0 自动选择，默认 0 / Parallel jobs; 0 = automatic (default: 0)",
        help_heading = "执行策略 / Execution"
    )]
    pub jobs: Option<usize>,
    #[arg(
        long,
        global = true,
        value_name = "SECONDS",
        help = "单条命令超时 / Per-command timeout",
        help_heading = "执行策略 / Execution"
    )]
    pub timeout_seconds: Option<u64>,
    #[arg(long,global=true,value_parser=["zh-CN","en-US"], help = "界面和报告语言，未指定时自动识别 / UI and report language (auto-detected if unset)", help_heading = "输出与语言 / Output")]
    pub lang: Option<String>,
    #[arg(
        long,
        global = true,
        help = "输出单个 JSON 报告 / Machine-readable report",
        help_heading = "输出与语言 / Output"
    )]
    pub json: bool,
    #[arg(
        long,
        global = true,
        help = "禁止交互 / Never prompt",
        help_heading = "执行策略 / Execution"
    )]
    pub non_interactive: bool,
    #[arg(
        long,
        global = true,
        help = "打开键盘向导 / Open keyboard wizard",
        help_heading = "执行策略 / Execution"
    )]
    pub interactive: bool,
    #[arg(
        long,
        global = true,
        help = "另存完整 JSON 报告；本机日志仍保留 / Also save the complete JSON report to this file",
        help_heading = "输出与语言 / Output"
    )]
    pub report_json: Option<PathBuf>,
    #[arg(
        short,
        long,
        global = true,
        help = "显示完整命令和详细日志 / Show commands and detailed logs",
        help_heading = "输出与语言 / Output"
    )]
    pub verbose: bool,
}
#[derive(Debug, Subcommand)]
pub enum Cmd {
    #[command(
        subcommand,
        about = "Agent 能力发现与 JSON 请求 / Agent discovery and structured requests"
    )]
    Agent(AgentCmd),
    #[command(about = "MCP 标准输入输出服务 / MCP stdio server")]
    Mcp,
    #[command(about = "分页查询协议目录（只读） / Query protocol catalog")]
    Catalog {
        #[arg(long)]
        query: Option<String>,
        #[arg(long, default_value_t = 0)]
        offset: usize,
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    #[command(
        about = "分页读取完整报告（不生成新日志） / Read a saved report without rotating logs"
    )]
    Report {
        #[arg(long)]
        path: PathBuf,
        #[arg(long, default_value = "summary", value_parser=["summary","events","diagnostics","selected","excluded","filtered","changes","outputs","import_only","documentation_inputs"])]
        section: String,
        #[arg(long, default_value_t = 0)]
        offset: usize,
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
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
        #[arg(long, help = "内部基准目录 / Internal baseline directory")]
        baseline: PathBuf,
        #[arg(long, help = "待比较产物目录 / Actual outputs directory")]
        actual: PathBuf,
    },
    #[command(
        subcommand,
        about = "查看、校验、初始化或迁移配置 / Manage configuration"
    )]
    Config(ConfigCmd),
    #[command(about = "安装当前二进制并注册 PATH / Install current executable")]
    Install {
        #[arg(
            long,
            help = "操作系统级安装，需要相应权限 / System-wide installation; requires permissions"
        )]
        system: bool,
        #[arg(
            long,
            help = "安装目录，默认用户本地应用目录 / Installation directory; defaults to local application data"
        )]
        dir: Option<PathBuf>,
    },
    #[command(about = "卸载程序 / Uninstall")]
    Uninstall {
        #[arg(
            long,
            help = "操作系统级安装，需要相应权限 / System-wide installation; requires permissions"
        )]
        system: bool,
        #[arg(long, help = "同时清除工具用户数据 / Also remove tool user data")]
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
pub enum AgentCmd {
    #[command(
        about = "机器可读的能力、参数 schema 与副作用 / Machine-readable capabilities and schemas"
    )]
    Capabilities,
    #[command(about = "执行 JSON 请求；- 表示标准输入 / Execute a JSON request; - reads stdin")]
    Run {
        #[arg(long, value_name = "FILE")]
        request: PathBuf,
    },
}
#[derive(Debug, Subcommand)]
pub enum ConfigCmd {
    #[command(
        about = "查看有效配置及路径 / Show effective configuration",
        after_help = "Example: proto-builder config show --explain --json"
    )]
    Show {
        #[arg(
            long,
            help = "解释配置来源与覆盖关系 / Explain configuration provenance"
        )]
        explain: bool,
    },
    #[command(about = "校验配置项与参数 / Validate configuration")]
    Check,
    #[command(about = "创建配置文件，不覆盖已有文件 / Create a configuration file")]
    Init {
        #[arg(
            long,
            help = "配置文件路径，默认项目根目录 / Configuration file path; defaults to project root"
        )]
        output: Option<PathBuf>,
    },
    #[command(about = "迁移旧配置（不调用旧脚本） / Migrate legacy configuration")]
    Migrate {
        #[arg(long, help = "仅检查迁移，不保存 / Check migration without saving")]
        check: bool,
    },
}
#[derive(Debug, Subcommand)]
pub enum UpdateCmd {
    #[command(about = "检查官方签名更新 / Check official releases")]
    Check,
    #[command(about = "下载、验签并替换程序 / Download, verify and update executable")]
    Apply {
        #[arg(long,value_parser=["stable","beta"])]
        channel: Option<String>,
    },
    #[command(
        about = "恢复更新前的程序版本，不恢复生成物 / Restore previous executable, not generated files"
    )]
    Rollback,
}
impl Cmd {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Agent(_) => "agent",
            Self::Mcp => "mcp",
            Self::Catalog { .. } => "catalog",
            Self::Report { .. } => "report",
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
