import type { AlertEvent } from "../types";
import "./Modal.css";

interface AlertModalProps {
  alert: AlertEvent;
  onClose: () => void;
}

export default function AlertModal({ alert, onClose }: AlertModalProps) {

  // 获取告警规则描述
  const getRuleDescription = (): string => {
    switch (alert.rule) {
      case "profit_threshold":
        return `盈利达到 ${alert.rule_value}%`;
      case "loss_threshold":
        return `亏损达到 ${alert.rule_value}%`;
      case "profit_drawdown_half":
        return "盈利回撤过半";
      default:
        return "未知告警";
    }
  };

  // 格式化盈亏比例
  const formatPnlRatio = (ratio: number): string => {
    const sign = ratio >= 0 ? "+" : "";
    return `${sign}${(ratio * 100).toFixed(2)}%`;
  };

  // 获取盈亏颜色
  const getPnlColor = (ratio: number): string => {
    return ratio >= 0 ? "#52c41a" : "#ff4d4f";
  };

  return (
    <div className="modal-overlay alert-overlay">
      <div className="modal-content alert-content">
        <div className="alert-header">
          <div className="alert-icon">⚠️</div>
          <h2 className="alert-title">股票告警</h2>
          <button className="modal-close" onClick={onClose}>
            ×
          </button>
        </div>
        <div className="alert-body">
          <div className="alert-stock-info">
            <div className="alert-stock-name">
              <span className="alert-stock-code">{alert.code}</span>
              <span className="alert-stock-name-text">{alert.name}</span>
            </div>
          </div>
          <div className="alert-rule">
            <span className="alert-rule-label">告警规则：</span>
            <span className="alert-rule-value">{getRuleDescription()}</span>
          </div>
          <div className="alert-price-info">
            <div className="alert-price-item">
              <span className="alert-price-label">当前价格：</span>
              <span className="alert-price-value">¥{alert.current_price.toFixed(2)}</span>
            </div>
            <div className="alert-price-item">
              <span className="alert-price-label">盈亏比例：</span>
              <span
                className="alert-price-value"
                style={{ color: getPnlColor(alert.pnl_ratio) }}
              >
                {formatPnlRatio(alert.pnl_ratio)}
              </span>
            </div>
            {alert.max_profit_ratio > 0 && (
              <div className="alert-price-item">
                <span className="alert-price-label">最高盈利：</span>
                <span className="alert-price-value" style={{ color: "#52c41a" }}>
                  +{(alert.max_profit_ratio * 100).toFixed(2)}%
                </span>
              </div>
            )}
          </div>
          <div className="alert-time">
            {new Date(alert.timestamp).toLocaleString("zh-CN")}
          </div>
        </div>
        <div className="alert-footer">
          <button className="btn btn-primary" onClick={onClose}>
            知道了
          </button>
        </div>
      </div>
    </div>
  );
}

