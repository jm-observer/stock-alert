import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import StockCard from "./components/StockCard";
import AddStockModal from "./components/AddStockModal";
import ConfigModal from "./components/ConfigModal";
import { api } from "./api";
import type { StockPosition } from "./types";
import "./App.css";

function App() {
  const [stocks, setStocks] = useState<StockPosition[]>([]);
  const [showAddModal, setShowAddModal] = useState(false);
  const [showConfigModal, setShowConfigModal] = useState(false);
  const [loading, setLoading] = useState(true);

  // 加载股票列表
  const loadStocks = async () => {
    try {
      const data = await api.listStocks();
      setStocks(data);
    } catch (error) {
      console.error("加载股票列表失败:", error);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadStocks();

    // 监听股票更新事件
    const unlisten = listen<StockPosition>("stock-update", (event) => {
      setStocks((prev) => {
        const index = prev.findIndex((s) => s.code === event.payload.code);
        if (index >= 0) {
          const updated = [...prev];
          updated[index] = event.payload;
          return updated;
        }
        return prev;
      });
    });

    return () => {
      unlisten.then((fn) => fn());
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

  // 切换启用状态
  const handleToggleEnabled = async (code: string, enabled: boolean) => {
    try {
      await api.setStockEnabled(code, enabled);
      await loadStocks();
    } catch (error) {
      console.error("更新股票状态失败:", error);
      alert("更新股票状态失败");
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
                onToggleEnabled={handleToggleEnabled}
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
    </div>
  );
}

export default App;

