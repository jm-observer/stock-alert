// 股票持仓信息
export interface StockPosition {
  id?: number;
  code: string;
  name: string;
  buy_price: number;
  buy_date: string;
  current_price?: number;
  highest_price_since_buy?: number;
  profit_threshold_level1_alerted: boolean;
  profit_threshold_level2_alerted: boolean;
  loss_threshold_alerted: boolean;
  profit_drawdown_half_alerted: boolean;
  created_at?: string;
  updated_at?: string;
}

// 股票更新事件
export interface StockUpdateEvent {
  position: StockPosition;
}

// 配置信息
export interface Config {
  db_path: string;
  trading_hours: TradingHoursConfig;
  timezone: string;
  quote_source: QuoteSourceConfig;
  alert: AlertConfig;
  notify: NotifyConfig;
}

export interface TradingHoursConfig {
  morning_start: string;
  morning_end: string;
  afternoon_start: string;
  afternoon_end: string;
}

export interface QuoteSourceConfig {
  type: "sse" | "test";
  [key: string]: any;
}

export interface AlertConfig {
  profit_thresholds: number[];
  loss_threshold?: number;
}

export interface NotifyConfig {
  sound_file?: string;
  sound_min_interval: number;
  log_file?: string;
}

// 告警事件
export interface AlertEvent {
  code: string;
  name: string;
  rule: "profit_threshold" | "loss_threshold" | "profit_drawdown_half";
  rule_value?: number; // 阈值百分比（仅用于 profit_threshold 和 loss_threshold）
  current_price: number;
  pnl_ratio: number; // 盈亏比例（小数，如 0.1 表示 10%）
  max_profit_ratio: number; // 最高盈利比例（小数）
  timestamp: string; // ISO 8601 格式的时间字符串
}

