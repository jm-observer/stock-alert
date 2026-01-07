import { invoke } from "@tauri-apps/api/core";
import type { StockPosition, Config } from "./types";

// Tauri命令封装
export const api = {
  // 添加股票
  addStock: async (
    code: string,
    name: string,
    buyPrice: number,
    buyDate: string
  ): Promise<number> => {
    return invoke("add_stock", { code, name, buyPrice, buyDate });
  },

  // 删除股票
  deleteStock: async (code: string): Promise<boolean> => {
    return invoke("delete_stock", { code });
  },

  // 更新股票
  updateStock: async (
    code: string,
    buyPrice: number,
    buyDate: string
  ): Promise<boolean> => {
    return invoke("update_stock", { code, buyPrice, buyDate });
  },

  // 获取所有股票
  listStocks: async (): Promise<StockPosition[]> => {
    return invoke("list_stocks");
  },

  // 获取配置
  getConfig: async (): Promise<Config> => {
    return invoke("get_config");
  },

  // 保存配置
  saveConfig: async (config: Config): Promise<void> => {
    return invoke("save_config", { config });
  },

  // 更新告警阈值
  updateAlertThresholds: async (
    profitThresholds: number[],
    lossThreshold?: number
  ): Promise<void> => {
    return invoke("update_alert_thresholds", {
      profitThresholds,
      lossThreshold,
    });
  },

  // 更新声音文件
  updateSoundFile: async (soundFile?: string): Promise<void> => {
    return invoke("update_sound_file", { soundFile });
  },
};

