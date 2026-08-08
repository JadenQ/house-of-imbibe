// net/ws.ts — 浏览器 WebSocket 实现 Transport；指数退避重连。无 phaser。dev-plan §2.3。
// 重连后服务端会自动重发 welcome + snapshot_full（新连接），故客户端无需自行补状态。
// 缓冲：onMessage 注册前到达的帧（如 handshake 即发的 welcome）不丢，注册时回放。

import type { Transport } from "./transport";
import { parseMsg, msg, type ClientMsg, type ServerMsg } from "../protocol/types";

// 把协议工具一并 re-export，方便 scene 从 net/ws 一站式 import。
export { parseMsg, msg, type ClientMsg, type ServerMsg };

const RECONNECT_MIN_MS = 500;
const RECONNECT_MAX_MS = 8000;

export class WsClient implements Transport {
  private ws: WebSocket | null = null;
  private messageCb: ((raw: string) => void) | null = null;
  private closeCb: ((code: number) => void) | null = null;
  private shouldReconnect = false;
  private reconnectDelay = RECONNECT_MIN_MS;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  // onMessage 注册前到达的帧先缓冲，注册时回放（避免丢 welcome）。
  private pending: string[] = [];

  /** 打开连接。可先于 onMessage 调用 connect —— 早到的帧会缓冲。 */
  connect(): void {
    this.shouldReconnect = true;
    this.reconnectDelay = RECONNECT_MIN_MS;
    this.openSocket();
  }

  private url(): string {
    const proto = location.protocol === "https:" ? "wss" : "ws";
    return `${proto}://${location.host}/ws/room`;
  }

  private openSocket(): void {
    const ws = new WebSocket(this.url());
    this.ws = ws;
    ws.onmessage = (ev: MessageEvent) => {
      const raw = typeof ev.data === "string" ? ev.data : String(ev.data);
      if (this.messageCb) {
        this.messageCb(raw);
      } else {
        this.pending.push(raw);
      }
    };
    ws.onclose = (ev: CloseEvent) => {
      this.closeCb?.(ev.code);
      if (this.shouldReconnect) {
        this.scheduleReconnect();
      }
    };
    ws.onerror = () => {
      // 错误随后必触发 onclose；重连交给 onclose 统一调度，避免重复。
    };
  }

  private scheduleReconnect(): void {
    if (this.reconnectTimer !== null) {
      clearTimeout(this.reconnectTimer);
    }
    const delay = this.reconnectDelay;
    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      if (this.shouldReconnect) {
        this.openSocket();
      }
    }, delay);
    this.reconnectDelay = Math.min(this.reconnectDelay * 2, RECONNECT_MAX_MS);
  }

  send(raw: string): void {
    // 仅在 OPEN 时发送，避免 CONNECTING 态抛 InvalidStateError（重连窗口内的 send 静默丢弃，
    // 重连后服务端会补发 snapshot_full，本地状态自动收敛）。
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(raw);
    }
  }

  /** 便捷发送：自动 JSON.stringify。 */
  sendMsg(m: ClientMsg): void {
    this.send(JSON.stringify(m));
  }

  onMessage(cb: (raw: string) => void): void {
    this.messageCb = cb;
    // 回放缓冲帧（如 handshake 即发的 welcome/backlog）
    while (this.pending.length > 0) {
      cb(this.pending.shift()!);
    }
  }

  onClose(cb: (code: number) => void): void {
    this.closeCb = cb;
  }

  close(): void {
    this.shouldReconnect = false;
    if (this.reconnectTimer !== null) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    this.pending = [];
    this.ws?.close();
    this.ws = null;
  }
}
