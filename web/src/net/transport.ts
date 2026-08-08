// net/transport.ts —— 传输层抽象，让 game-state 可在无真 WebSocket 时单测。
// 不 import phaser（dev-plan §2.3）。

export interface Transport {
  send(raw: string): void;
  onMessage(cb: (raw: string) => void): void;
  onClose(cb: (code: number) => void): void;
  close(): void;
}
