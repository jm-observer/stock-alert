import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import StockCard from "./components/StockCard";
import AddStockModal from "./components/AddStockModal";
import ConfigModal from "./components/ConfigModal";
import AlertModal from "./components/AlertModal";
import { api } from "./api";
import type { StockPosition, AlertEvent, StockUpdateEvent } from "./types";
import "./App.css";

function App() {
  const [stocks, setStocks] = useState<StockPosition[]>([]);
  const [showAddModal, setShowAddModal] = useState(false);
  const [showConfigModal, setShowConfigModal] = useState(false);
  const [loading, setLoading] = useState(true);
  const [currentAlert, setCurrentAlert] = useState<AlertEvent | null>(null);

  // 加载股票列表
  const loadStocks = async () => {
    console.log("[App] 开始加载股票列表...");
    try {
      const data = await api.listStocks();
      console.log("[App] ✅ 股票列表加载成功:", data);
      setStocks(data);
    } catch (error) {
      console.error("[App] ❌ 加载股票列表失败:", error);
    } finally {
      setLoading(false);
      console.log("[App] 加载状态设置为完成");
    }
  };

  useEffect(() => {
    loadStocks();

    // 监听股票更新事件
    console.log("[App] 开始注册股票更新事件监听器...");
    const unlistenStockUpdatePromise = listen<StockUpdateEvent>("stock-update", (event) => {
      console.log("[App] ✅ 收到股票更新事件:", event);
      console.log("[App] 事件数据:", event.payload);
      const position = event.payload.position;
      console.log("[App] 股票持仓信息:", position);
      setStocks((prev) => {
        console.log("[App] 当前股票列表:", prev);
        const index = prev.findIndex((s) => s.code === position.code);
        console.log("[App] 找到股票索引:", index, "股票代码:", position.code);
        if (index >= 0) {
          const updated = [...prev];
          updated[index] = position;
          console.log("[App] ✅ 更新后的股票列表:", updated);
          return updated;
        }
        console.log("[App] ⚠️ 未找到匹配的股票，保持原列表");
        return prev;
      });
    });

    unlistenStockUpdatePromise
      .then(() => {
        console.log("[App] ✅ 股票更新事件监听器注册成功");
      })
      .catch((error) => {
        console.error("[App] ❌ 注册股票更新事件监听器失败:", error);
      });

    // 监听告警事件
    console.log("[App] 开始注册告警事件监听器...");
    const unlistenAlertPromise = listen<AlertEvent>("stock-alert", (event) => {
      console.log("[App] ✅ 收到告警事件:", event);
      console.log("[App] 告警数据:", event.payload);
      setCurrentAlert(event.payload);
    });

    unlistenAlertPromise
      .then(() => {
        console.log("[App] ✅ 告警事件监听器注册成功");
      })
      .catch((error) => {
        console.error("[App] ❌ 注册告警事件监听器失败:", error);
      });

    return () => {
      console.log("[App] 清理事件监听器...");
      unlistenStockUpdatePromise
        .then((fn) => {
          if (fn) fn();
        })
        .catch((e) => console.error("[App] 清理股票更新监听器失败:", e));
      unlistenAlertPromise
        .then((fn) => {
          if (fn) fn();
        })
        .catch((e) => console.error("[App] 清理告警监听器失败:", e));
    };
  }, []);

  // 删除股票
  const handleDelete = async (code: string) => {
    if (confirm(`确定要删除股票 ${code} 吗？`)) {
      try {
        await api.deleteStock(code);
        await loadStocks();
      } catch (error) {
        console.error("删除股票失败:", error);
        alert("删除股票失败");
      }
    }
  };



  if (loading) {
    return (
      <div className="app-loading">
        <div>加载中...</div>
      </div>
    );
  }

  return (
    <div className="app">
      <header className="app-header">
        <h1>股票提醒</h1>
        <div className="header-actions">
          <button className="btn btn-primary" onClick={() => setShowAddModal(true)}>
            添加股票
          </button>
          <button className="btn btn-secondary" onClick={() => setShowConfigModal(true)}>
            配置
          </button>
        </div>
      </header>

      <main className="app-main">
        {stocks.length === 0 ? (
          <div className="empty-state">
            <p>还没有添加股票，点击"添加股票"开始监控</p>
          </div>
        ) : (
          <div className="stock-grid">
            {stocks.map((stock) => (
              <StockCard
                key={stock.code}
                stock={stock}
                onDelete={handleDelete}
              />
            ))}
          </div>
        )}
      </main>

      {showAddModal && (
        <AddStockModal
          onClose={() => setShowAddModal(false)}
          onSuccess={() => {
            setShowAddModal(false);
            loadStocks();
          }}
        />
      )}

      {showConfigModal && (
        <ConfigModal
          onClose={() => setShowConfigModal(false)}
          onSuccess={() => {
            setShowConfigModal(false);
          }}
        />
      )}

      {currentAlert && (
        <AlertModal
          alert={currentAlert}
          onClose={() => setCurrentAlert(null)}
        />
      )}
    </div>
  );
}

export default App;

