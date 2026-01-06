// 股票持仓信息
export interface StockPosition {
  id?: number;
  code: string;
  name: string;
  buy_price: number;
  buy_date: string;
  current_price?: number;
  highest_price_since_buy?: number;
  enabled: boolean;
  profit_threshold_level1_alerted: boolean;
  profit_threshold_level2_alerted: boolean;
  loss_threshold_alerted: boolean;
  profit_drawdown_half_alerted: boolean;
  created_at?: string;
  updated_at?: string;
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

