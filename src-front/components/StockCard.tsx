import { useState } from "react";
import type { StockPosition } from "../types";
import "./StockCard.css";

interface StockCardProps {
  stock: StockPosition;
  onDelete: (code: string) => void;
}

export default function StockCard({ stock, onDelete }: StockCardProps) {
  const [isDeleting, setIsDeleting] = useState(false);

  const pnlRatio = stock.current_price
    ? ((stock.current_price - stock.buy_price) / stock.buy_price) * 100
    : null;

  const maxProfitRatio = stock.highest_price_since_buy
    ? ((stock.highest_price_since_buy - stock.buy_price) / stock.buy_price) * 100
    : null;

  const pnlColor = pnlRatio === null ? "#999" : pnlRatio >= 0 ? "#f5222d" : "#52c41a";

  const formatDate = (dateStr?: string) => {
    if (!dateStr) return "N/A";
    return dateStr.split("T")[0];
  };

  const formatPrice = (price?: number) => {
    return price !== undefined ? price.toFixed(2) : "N/A";
  };

  return (
    <div className={`stock-card`}>
      <div className="stock-card-header">
        <div className="stock-title">
          <span className="stock-code">{stock.code}</span>
          <h3>{stock.name}</h3>
        </div>
        <div className="stock-actions">
    
          <button
            className="btn-delete"
            onClick={() => {
              if (!isDeleting) {
                setIsDeleting(true);
                onDelete(stock.code);
              }
            }}
            disabled={isDeleting}
          >
            {isDeleting ? "删除中..." : "删除"}
          </button>
        </div>
      </div>

      <div className="stock-card-body">
        <div className="stock-info-row">
          <span className="label">买入价:</span>
          <span className="value">¥{formatPrice(stock.buy_price)}</span>
        </div>
        <div className="stock-info-row">
          <span className="label">买入日期:</span>
          <span className="value">{formatDate(stock.buy_date)}</span>
        </div>
        <div className="stock-info-row">
          <span className="label">当前价:</span>
          <span className="value">¥{formatPrice(stock.current_price)}</span>
        </div>
        <div className="stock-info-row">
          <span className="label">最高价:</span>
          <span className="value">¥{formatPrice(stock.highest_price_since_buy)}</span>
        </div>
        <div className="stock-info-row">
          <span className="label">盈亏比例:</span>
          <span className="value" style={{ color: pnlColor }}>
            {pnlRatio !== null ? `${pnlRatio >= 0 ? "+" : ""}${pnlRatio.toFixed(2)}%` : "N/A"}
          </span>
        </div>
        {maxProfitRatio !== null && (
          <div className="stock-info-row">
            <span className="label">最高盈利:</span>
            <span className="value" style={{ color: "#f5222d" }}>
              {maxProfitRatio >= 0 ? "+" : ""}{maxProfitRatio.toFixed(2)}%
            </span>
          </div>
        )}
      </div>

      <div className="stock-card-footer">
        <div className="footer-content">
          <div className="alert-status">
            <span className="alert-item">
              盈1级: {stock.profit_threshold_level1_alerted ? "✓" : "✗"}
            </span>
            <span className="alert-item">
              盈2级: {stock.profit_threshold_level2_alerted ? "✓" : "✗"}
            </span>
            <span className="alert-item">
              亏: {stock.loss_threshold_alerted ? "✓" : "✗"}
            </span>
            <span className="alert-item">
              回撤: {stock.profit_drawdown_half_alerted ? "✓" : "✗"}
            </span>
          </div>
          {stock.updated_at && (
            <div className="update-time">
              更新: {new Date(stock.updated_at).toLocaleString("zh-CN")}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

