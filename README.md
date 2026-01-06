# 股票价格提醒程序

一个本地运行的股票价格提醒程序，支持通过SSE实时获取股票价格，并在满足条件时播放声音提醒。

## 功能特性

1. **数据持久化**：使用SQLite数据库存储股票持仓信息，程序重启后数据不丢失
2. **实时行情**：优先使用SSE（Server-Sent Events）获取实时价格，支持自动重连
3. **交易时段控制**：仅在交易时段（09:30-11:30, 13:00-15:00）内监控，非交易时段自动停止
4. **告警规则**：
   - 盈利阈值提醒（默认：+10%, +20%）
   - 亏损阈值提醒（可选：-10%, -20%）
   - 盈利回撤过半提醒
5. **声音提醒**：Windows系统提示音或自定义声音文件
6. **日志记录**：所有告警事件都会记录到日志文件

## 安装

```bash
cargo build --release
```

## 使用方法

### 1. 首次运行

首次运行会自动创建 `config.toml` 配置文件，你可以根据需要修改配置。

### 2. 管理股票持仓

程序提供了命令行工具来管理股票持仓：

```bash
# 添加股票持仓
./stock-alert add 000001 平安银行 10.50 2024-01-01

# 列出所有持仓
./stock-alert list

# 只列出启用的持仓
./stock-alert list --enabled-only

# 查看单个股票信息
./stock-alert show 000001

# 更新买入价格和日期
./stock-alert update 000001 11.00 2024-01-15

# 启用/禁用监控
./stock-alert enable 000001 true
./stock-alert enable 000001 false

# 删除股票持仓
./stock-alert delete 000001
```

或者你也可以直接使用SQLite命令行工具：

```sql
-- 连接到数据库
sqlite3 stock_alert.db

-- 添加股票持仓
INSERT INTO stock_positions (code, name, buy_price, buy_date, enabled)
VALUES ('000001', '平安银行', 10.50, '2024-01-01', 1);

-- 查看所有持仓
SELECT * FROM stock_positions;

-- 启用/禁用监控
UPDATE stock_positions SET enabled = 1 WHERE code = '000001';
```

### 3. 运行程序

```bash
cargo run --release
# 或
./target/release/stock-alert
```

程序会在交易时段自动开始监控，非交易时段自动停止。

## 配置说明

`config.toml` 配置文件示例：

```toml
[db_path]
path = "stock_alert.db"

[trading_hours]
morning_start = "09:30"
morning_end = "11:30"
afternoon_start = "13:00"
afternoon_end = "15:00"

timezone = "Asia/Shanghai"

[quote_source]
sse_url_template = "https://46.push2.eastmoney.com/api/qt/stock/sse"
sse_fields = "f58,f107,f57,f43,f59,f169,f170,f152,f46,f60,f44,f45,f47,f48,f19,f532,f39,f161,f49,f171,f50,f86,f600,f601,f154,f84,f85,f168,f108,f116,f167,f164,f92,f71,f117,f292,f113,f114,f115,f119,f120,f121,f122,f296"
sse_mpi = 1000
sse_ut = "fa5fd1943c7b386f172d6893dbfba10b"
enable_polling_fallback = true
polling_interval = 2
polling_url_template = "https://push2.eastmoney.com/api/qt/stock/get"
max_reconnect_attempts = 0
reconnect_initial_delay = 1
reconnect_max_delay = 60

[alert]
profit_thresholds = [0.1, 0.2]  # 10%, 20%
enable_loss_thresholds = false
loss_thresholds = [-0.1, -0.2]
enable_profit_drawdown_half = true

[notify]
sound_file = null  # 可选：自定义声音文件路径
sound_min_interval = 1  # 声音播放最小间隔（秒）
log_file = "alerts.log"
```

## 告警规则说明

### 盈利阈值提醒

当股票盈亏比例从阈值以下穿越到阈值以上时触发：
- 默认阈值：+10%, +20%
- 去重策略：只有当盈亏比例回落到阈值以下，才允许再次触发

### 亏损阈值提醒

当股票盈亏比例从阈值以上穿越到阈值以下时触发：
- 默认阈值：-10%, -20%（默认禁用）
- 去重策略：只有当盈亏比例回升到阈值以上，才允许再次触发

### 盈利回撤过半提醒

当股票从最高盈利回撤超过一半时触发：
- 例如：最高盈利20%，当前盈利降到10%以下时触发
- 去重策略：只有当盈利再次回到一半以上，才允许再次触发

## 日志格式

告警日志记录在 `alerts.log` 文件中，格式如下：

```
[2024-01-15 10:30:25] 平安银行 (000001) - 盈利阈值 10.0% - 当前价: 11.55, 盈亏: 10.00%, 最高盈利: 12.00%
```

## 注意事项

1. **股票代码格式**：使用6位数字代码，如 `000001`（深圳）、`600000`（上海）
2. **交易时段**：程序仅在交易时段内监控，非交易时段不会请求行情
3. **SSE连接**：如果SSE连接失败，会自动切换到轮询模式（如果启用）
4. **声音播放**：Windows系统默认使用系统提示音，也可以配置自定义声音文件
5. **数据库备份**：建议定期备份 `stock_alert.db` 文件

## 开发说明

### 项目结构

```
stock-alert/
├── src/
│   ├── main.rs          # 程序入口
│   ├── app.rs           # 应用主逻辑
│   ├── config.rs        # 配置管理
│   ├── storage.rs       # 数据持久化
│   ├── quote.rs         # 行情数据源
│   ├── scheduler.rs     # 交易时段调度
│   ├── engine.rs        # 告警引擎
│   ├── notify.rs        # 通知模块
│   └── models.rs        # 数据模型
├── Cargo.toml
└── README.md
```

### 模块说明

- **storage**：SQLite数据库操作，提供CRUD接口
- **quote**：行情数据源抽象，支持SSE和轮询两种方式
- **scheduler**：交易时段判断和调度
- **engine**：告警规则引擎，实现穿越触发和去重逻辑
- **notify**：声音播放和日志记录

## 许可证

MIT License

