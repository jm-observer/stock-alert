import { useState, useEffect } from "react";
import { api } from "../api";
import type { Config } from "../types";
import "./Modal.css";

interface ConfigModalProps {
  onClose: () => void;
  onSuccess: () => void;
}

export default function ConfigModal({ onClose, onSuccess }: ConfigModalProps) {
  const [config, setConfig] = useState<Config | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    loadConfig();
  }, []);

  const loadConfig = async () => {
    try {
      const data = await api.getConfig();
      setConfig(data);
    } catch (err: any) {
      setError(err.message || "加载配置失败");
    } finally {
      setLoading(false);
    }
  };

  const handleSave = async () => {
    if (!config) return;

    setSaving(true);
    setError("");

    try {
      // 更新告警阈值
      await api.updateAlertThresholds(
        config.alert.profit_thresholds,
        config.alert.loss_threshold
      );

      // 更新声音文件
      await api.updateSoundFile(config.notify.sound_file);

      onSuccess();
    } catch (err: any) {
      setError(err.message || "保存配置失败");
    } finally {
      setSaving(false);
    }
  };

  const updateProfitThreshold = (index: number, value: string) => {
    if (!config) return;
    const numValue = parseFloat(value);
    if (isNaN(numValue)) return;

    const newThresholds = [...config.alert.profit_thresholds];
    newThresholds[index] = numValue; // 转换为小数
    setConfig({
      ...config,
      alert: { ...config.alert, profit_thresholds: newThresholds },
    });
  };

  const updateLossThreshold = (value: string) => {
    if (!config) return;
    if (value === "") {
      setConfig({
        ...config,
        alert: { ...config.alert, loss_threshold: undefined },
      });
      return;
    }
    const numValue = parseFloat(value);
    if (isNaN(numValue)) return;
    setConfig({
      ...config,
      alert: { ...config.alert, loss_threshold: numValue },
    });
  };

  const updateSoundFile = (value: string) => {
    if (!config) return;
    setConfig({
      ...config,
      notify: { ...config.notify, sound_file: value || undefined },
    });
  };

  if (loading) {
    return (
      <div className="modal-overlay" onClick={onClose}>
        <div className="modal-content" onClick={(e) => e.stopPropagation()}>
          <div className="modal-body">
            <div>加载配置中...</div>
          </div>
        </div>
      </div>
    );
  }

  if (!config) {
    return null;
  }

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>配置</h2>
          <button className="modal-close" onClick={onClose}>
            ×
          </button>
        </div>
        <div className="modal-body">
          {error && <div className="error-message">{error}</div>}
          <div className="config-section">
            <h3>告警阈值</h3>
            <div className="form-group">
              <label>盈利阈值1级 (%)</label>
              <input
                type="number"
                step="0.1"
                value={(config.alert.profit_thresholds[0]).toFixed(1)}
                onChange={(e) => updateProfitThreshold(0, e.target.value)}
              />
            </div>
            {config.alert.profit_thresholds.length > 1 && (
              <div className="form-group">
                <label>盈利阈值2级 (%)</label>
                <input
                  type="number"
                  step="0.1"
                  value={(config.alert.profit_thresholds[1]).toFixed(1)}
                  onChange={(e) => updateProfitThreshold(1, e.target.value)}
                />
              </div>
            )}
            <div className="form-group">
              <label>亏损阈值 (%)</label>
              <input
                type="number"
                step="0.1"
                value={
                  config.alert.loss_threshold
                    ? (config.alert.loss_threshold).toFixed(1)
                    : ""
                }
                onChange={(e) => updateLossThreshold(e.target.value)}
                placeholder="留空表示不启用"
              />
            </div>
          </div>
          <div className="config-section">
            <h3>通知设置</h3>
            <div className="form-group">
              <label>声音文件路径</label>
              <input
                type="text"
                value={config.notify.sound_file || ""}
                onChange={(e) => updateSoundFile(e.target.value)}
                placeholder="留空表示使用系统提示音"
              />
            </div>
          </div>
        </div>
        <div className="modal-footer">
          <button type="button" className="btn btn-secondary" onClick={onClose}>
            取消
          </button>
          <button
            type="button"
            className="btn btn-primary"
            onClick={handleSave}
            disabled={saving}
          >
            {saving ? "保存中..." : "保存"}
          </button>
        </div>
      </div>
    </div>
  );
}

