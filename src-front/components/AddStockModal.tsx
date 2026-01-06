import { useState } from "react";
import { api } from "../api";
import "./Modal.css";

interface AddStockModalProps {
  onClose: () => void;
  onSuccess: () => void;
}

export default function AddStockModal({ onClose, onSuccess }: AddStockModalProps) {
  const [code, setCode] = useState("");
  const [name, setName] = useState("");
  const [buyPrice, setBuyPrice] = useState("");
  // 默认买入日期为当前日期
  const [buyDate, setBuyDate] = useState(() => {
    const today = new Date();
    return today.toISOString().split("T")[0];
  });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError("");

    if (!code || !name || !buyPrice || !buyDate) {
      setError("请填写所有字段");
      return;
    }

    const price = parseFloat(buyPrice);
    if (isNaN(price) || price <= 0) {
      setError("买入价必须是大于0的数字");
      return;
    }

    setLoading(true);
    try {
      await api.addStock(code, name, price, buyDate);
      onSuccess();
    } catch (err: any) {
      setError(err.message || "添加股票失败");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" onClick={(e) => e.stopPropagation()}>
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
              <label>
                买入日期<span className="required-mark">*</span>
              </label>
              <input
                type="date"
                value={buyDate}
                onChange={(e) => setBuyDate(e.target.value)}
                required
              />
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

