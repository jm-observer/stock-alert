import { useState, useEffect } from "react";
import { api } from "../api";
import type { StockPosition, ThresholdType } from "../types";
import "./Modal.css";

interface EditStockModalProps {
  stock: StockPosition;
  onClose: () => void;
  onSuccess: () => void;
}

export default function EditStockModal({ stock, onClose, onSuccess }: EditStockModalProps) {
  const [buyPrice, setBuyPrice] = useState(stock.buy_price.toString());
  
  // 盈利阈值1级
  const [profitThreshold1Type, setProfitThreshold1Type] = useState<"Ratio" | "Price">(
    stock.profit_threshold1?.type || "Ratio"
  );
  const [profitThreshold1Value, setProfitThreshold1Value] = useState(
    stock.profit_threshold1?.value.toString() || ""
  );
  
  // 盈利阈值2级
  const [profitThreshold2Type, setProfitThreshold2Type] = useState<"Ratio" | "Price">(
    stock.profit_threshold2?.type || "Ratio"
  );
  const [profitThreshold2Value, setProfitThreshold2Value] = useState(
    stock.profit_threshold2?.value.toString() || ""
  );
  
  // 亏损阈值
  const [lossThresholdType, setLossThresholdType] = useState<"Ratio" | "Price">(
    stock.loss_threshold?.type || "Ratio"
  );
  const [lossThresholdValue, setLossThresholdValue] = useState(
    stock.loss_threshold?.value.toString() || ""
  );
  
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    // 初始化表单数据
    if (stock.profit_threshold1) {
      setProfitThreshold1Type(stock.profit_threshold1.type);
      setProfitThreshold1Value(stock.profit_threshold1.value.toString());
    }
    if (stock.profit_threshold2) {
      setProfitThreshold2Type(stock.profit_threshold2.type);
      setProfitThreshold2Value(stock.profit_threshold2.value.toString());
    }
    if (stock.loss_threshold) {
      setLossThresholdType(stock.loss_threshold.type);
      setLossThresholdValue(stock.loss_threshold.value.toString());
    }
  }, [stock]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError("");

    if (!buyPrice) {
      setError("请填写买入价");
      return;
    }

    const price = parseFloat(buyPrice);
    if (isNaN(price) || price <= 0) {
      setError("买入价必须是大于0的数字");
      return;
    }

    // 构建阈值对象
    if (!profitThreshold1Value || !profitThreshold2Value || !lossThresholdValue) {
      setError("请填写所有阈值字段");
      return;
    }

    // 如果是比例模式，值自动调整为整数
    const profitThreshold1: ThresholdType = {
      type: profitThreshold1Type,
      value: profitThreshold1Type === "Ratio" 
        ? Math.round(parseFloat(profitThreshold1Value))
        : parseFloat(profitThreshold1Value),
    };
    
    const profitThreshold2: ThresholdType = {
      type: profitThreshold2Type,
      value: profitThreshold2Type === "Ratio"
        ? Math.round(parseFloat(profitThreshold2Value))
        : parseFloat(profitThreshold2Value),
    };
    
    const lossThreshold: ThresholdType = {
      type: lossThresholdType,
      value: lossThresholdType === "Ratio"
        ? Math.round(parseFloat(lossThresholdValue))
        : parseFloat(lossThresholdValue),
    };

    // 验证阈值
    if (isNaN(profitThreshold1.value) || profitThreshold1.value <= 0) {
      setError("盈利阈值1级必须是大于0的数字");
      return;
    }
    if (isNaN(profitThreshold2.value) || profitThreshold2.value <= 0) {
      setError("盈利阈值2级必须是大于0的数字");
      return;
    }
    if (isNaN(lossThreshold.value) || lossThreshold.value <= 0) {
      setError("亏损阈值必须是大于0的数字");
      return;
    }

    setLoading(true);
    try {
      await api.updateStock(
        stock.code,
        price,
        profitThreshold1,
        profitThreshold2,
        lossThreshold
      );
      onSuccess();
    } catch (err: any) {
      setError(err.message || "更新股票失败");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>编辑股票 - {stock.name} ({stock.code})</h2>
          <button className="modal-close" onClick={onClose}>
            ×
          </button>
        </div>
        <form onSubmit={handleSubmit}>
          <div className="modal-body">
            {error && <div className="error-message">{error}</div>}
            <div className="form-group">
              <label>
                买入价<span className="required-mark">*</span>
              </label>
              <input
                type="number"
                step="0.01"
                value={buyPrice}
                onChange={(e) => setBuyPrice(e.target.value)}
                placeholder="例如: 10.50"
                required
              />
            </div>
            
            <div className="form-group">
              <label>盈利阈值1级<span className="required-mark">*</span></label>
              <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
                <select
                  value={profitThreshold1Type}
                  onChange={(e) => setProfitThreshold1Type(e.target.value as "Ratio" | "Price")}
                  style={{ flex: "0 0 100px" }}
                >
                  <option value="Ratio">百分比</option>
                  <option value="Price">价格</option>
                </select>
                <input
                  type="number"
                  step={profitThreshold1Type === "Ratio" ? "1" : "0.01"}
                  value={profitThreshold1Value}
                  onChange={(e) => setProfitThreshold1Value(e.target.value)}
                  placeholder={profitThreshold1Type === "Ratio" ? "例如: 10 (表示10%)" : "例如: 15.50"}
                  style={{ flex: "1" }}
                  required
                />
              </div>
            </div>
            
            <div className="form-group">
              <label>盈利阈值2级<span className="required-mark">*</span></label>
              <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
                <select
                  value={profitThreshold2Type}
                  onChange={(e) => setProfitThreshold2Type(e.target.value as "Ratio" | "Price")}
                  style={{ flex: "0 0 100px" }}
                >
                  <option value="Ratio">百分比</option>
                  <option value="Price">价格</option>
                </select>
                <input
                  type="number"
                  step={profitThreshold2Type === "Ratio" ? "1" : "0.01"}
                  value={profitThreshold2Value}
                  onChange={(e) => setProfitThreshold2Value(e.target.value)}
                  placeholder={profitThreshold2Type === "Ratio" ? "例如: 20 (表示20%)" : "例如: 18.00"}
                  style={{ flex: "1" }}
                  required
                />
              </div>
            </div>
            
            <div className="form-group">
              <label>亏损阈值<span className="required-mark">*</span></label>
              <div style={{ display: "flex", gap: "8px", alignItems: "center" }}>
                <select
                  value={lossThresholdType}
                  onChange={(e) => setLossThresholdType(e.target.value as "Ratio" | "Price")}
                  style={{ flex: "0 0 100px" }}
                >
                  <option value="Ratio">百分比</option>
                  <option value="Price">价格</option>
                </select>
                <input
                  type="number"
                  step={lossThresholdType === "Ratio" ? "1" : "0.01"}
                  value={lossThresholdValue}
                  onChange={(e) => setLossThresholdValue(e.target.value)}
                  placeholder={lossThresholdType === "Ratio" ? "例如: 10 (表示10%)" : "例如: 8.00"}
                  style={{ flex: "1" }}
                  required
                />
              </div>
            </div>
          </div>
          <div className="modal-footer">
            <button type="button" className="btn btn-secondary" onClick={onClose}>
              取消
            </button>
            <button type="submit" className="btn btn-primary" disabled={loading}>
              {loading ? "更新中..." : "更新"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

