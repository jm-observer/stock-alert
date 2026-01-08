import { useState } from "react";
import { api } from "../api";
import type { ThresholdType } from "../types";
import "./Modal.css";

interface AddStockModalProps {
  onClose: () => void;
  onSuccess: () => void;
}

export default function AddStockModal({ onClose, onSuccess }: AddStockModalProps) {
  const [code, setCode] = useState("");
  const [name, setName] = useState("");
  const [buyPrice, setBuyPrice] = useState("");
  
  // 盈利阈值1级
  const [profitThreshold1Type, setProfitThreshold1Type] = useState<"Ratio" | "Price">("Ratio");
  const [profitThreshold1Value, setProfitThreshold1Value] = useState("");
  
  // 盈利阈值2级
  const [profitThreshold2Type, setProfitThreshold2Type] = useState<"Ratio" | "Price">("Ratio");
  const [profitThreshold2Value, setProfitThreshold2Value] = useState("");
  
  // 亏损阈值
  const [lossThresholdType, setLossThresholdType] = useState<"Ratio" | "Price">("Ratio");
  const [lossThresholdValue, setLossThresholdValue] = useState("");
  
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError("");

    if (!code || !name || !buyPrice) {
      setError("请填写所有字段");
      return;
    }

    const price = parseFloat(buyPrice);
    if (isNaN(price) || price <= 0) {
      setError("买入价必须是大于0的数字");
      return;
    }

    // 构建阈值对象
    // 如果是比例模式，值自动调整为整数
    const profitThreshold1: ThresholdType | undefined = profitThreshold1Value
      ? {
          type: profitThreshold1Type,
          value: profitThreshold1Type === "Ratio"
            ? Math.round(parseFloat(profitThreshold1Value))
            : parseFloat(profitThreshold1Value),
        }
      : undefined;
    
    const profitThreshold2: ThresholdType | undefined = profitThreshold2Value
      ? {
          type: profitThreshold2Type,
          value: profitThreshold2Type === "Ratio"
            ? Math.round(parseFloat(profitThreshold2Value))
            : parseFloat(profitThreshold2Value),
        }
      : undefined;
    
    const lossThreshold: ThresholdType | undefined = lossThresholdValue
      ? {
          type: lossThresholdType,
          value: lossThresholdType === "Ratio"
            ? Math.round(parseFloat(lossThresholdValue))
            : parseFloat(lossThresholdValue),
        }
      : undefined;

    // 验证阈值
    if (profitThreshold1 && (isNaN(profitThreshold1.value) || profitThreshold1.value <= 0)) {
      setError("盈利阈值1级必须是大于0的数字");
      return;
    }
    if (profitThreshold2 && (isNaN(profitThreshold2.value) || profitThreshold2.value <= 0)) {
      setError("盈利阈值2级必须是大于0的数字");
      return;
    }
    if (lossThreshold && (isNaN(lossThreshold.value) || lossThreshold.value <= 0)) {
      setError("亏损阈值必须是大于0的数字");
      return;
    }

    setLoading(true);
    try {
      await api.addStock(code, name, price, profitThreshold1, profitThreshold2, lossThreshold);
      onSuccess();
    } catch (err: any) {
      setError(err.message || "添加股票失败");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="modal-overlay">
      <div className="modal-content">
        <div className="modal-header">
          <h2>添加股票</h2>
          <button className="modal-close" onClick={onClose}>
            ×
          </button>
        </div>
        <form onSubmit={handleSubmit}>
          <div className="modal-body">
            {error && <div className="error-message">{error}</div>}
            <div className="form-group">
              <label>
                股票代码<span className="required-mark">*</span>
              </label>
              <input
                type="text"
                value={code}
                onChange={(e) => setCode(e.target.value)}
                placeholder="例如: 000001"
                required
              />
            </div>
            <div className="form-group">
              <label>
                股票名称<span className="required-mark">*</span>
              </label>
              <input
                type="text"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="例如: 平安银行"
                required
              />
            </div>
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
              <label>盈利阈值1级（可选）</label>
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
                />
              </div>
            </div>
            
            <div className="form-group">
              <label>盈利阈值2级（可选）</label>
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
                />
              </div>
            </div>
            
            <div className="form-group">
              <label>亏损阈值（可选）</label>
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
                />
              </div>
            </div>
          </div>
          <div className="modal-footer">
            <button type="button" className="btn btn-secondary" onClick={onClose}>
              取消
            </button>
            <button type="submit" className="btn btn-primary" disabled={loading}>
              {loading ? "添加中..." : "添加"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

