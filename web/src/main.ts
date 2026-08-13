// 启动流程：登录 → （无形象则）创建形象 → 进入酒吧
import Phaser from "phaser";
import { api, type AvatarData, type Me } from "./net/api";
import { showLogin } from "./ui/login";
import { showAvatarCreate, showGenerationCenter } from "./ui/avatarCreate";
import { loadModularLocal } from "./ui/avatarBuilder";
import { showMenu, isMenuOpen, menuBlocksInteract } from "./ui/menu";
import { createChatPanel } from "./ui/chat";
import { createTouchControls } from "./ui/touch";
import { showAdminConsole } from "./ui/admin";
import { WsClient } from "./net/ws";
import { msg } from "./protocol/types";
import { BarScene } from "./scene/BarScene";

const LOGICAL_W = 240;
const LOGICAL_H = 160;

/** 整数缩放：能放下的最大整数倍 */
function fitZoom(): number {
  const z = Math.min(window.innerWidth / LOGICAL_W, (window.innerHeight - 8) / LOGICAL_H);
  return Math.max(1, Math.floor(z));
}

function startGame(me: Me, avatar: AvatarData) {
  const hud = document.getElementById("hud")!;
  hud.style.display = "block";
  document.getElementById("hud-user")!.textContent = me.username;

  // 聊天侧栏
  const gameEl = document.getElementById("game")!;
  const chat = createChatPanel(gameEl);

  // 触控层（DOM overlay，仅触控设备显示；桌面为 undefined 走键盘回退）
  const touch = createTouchControls(document.getElementById("app")!);

  // WS 传输 + 状态（scene 内部消费消息）
  const transport = new WsClient();
  transport.connect();
  chat.onSend((text) => transport.send(JSON.stringify(msg.chat(text))));

  const game = new Phaser.Game({
    type: Phaser.AUTO,
    parent: "game",
    width: LOGICAL_W,
    height: LOGICAL_H,
    zoom: fitZoom(),
    pixelArt: true,
    roundPixels: true,
    backgroundColor: "#14100e",
    scene: [],
  });
  game.scene.add("bar", BarScene, true, {
    avatar,
    transport,
    selfId: me.id,
    chatPanel: chat.panel,
    touch,
  });

  window.addEventListener("resize", () => game.scale.setZoom(fitZoom()));

  // scene → DOM 事件桥
  window.addEventListener("hoi:interact", ((e: CustomEvent<string>) => {
    if (e.detail === "menu" && !menuBlocksInteract()) void showMenu();
  }) as EventListener);
  window.addEventListener("hoi:hint", ((e: CustomEvent<string | null>) => {
    const toast = document.getElementById("toast")!;
    if (e.detail && !isMenuOpen()) {
      toast.textContent = e.detail;
      toast.style.display = "block";
    } else {
      toast.style.display = "none";
    }
  }) as EventListener);

  // HUD
  document.getElementById("hud-logout")!.onclick = async () => {
    await api.logout();
    location.reload();
  };
  document.getElementById("hud-edit")!.onclick = () => {
    void showAvatarCreate(avatar).then(() => location.reload());
  };
  // 生成中心入口（查看历史/进行中的形象生成 job，非阻塞轮询）
  document.getElementById("hud-gencenter")!.onclick = () => showGenerationCenter();
  // admin 入口：仅 is_admin 显示（Design 2 独立管理台，DOM 非 Phaser）
  if (me.is_admin) {
    const adminBtn = document.getElementById("hud-admin")!;
    adminBtn.style.display = "";
    adminBtn.onclick = () => showAdminConsole(() => { /* 关闭后回到游戏，不 reload */ });
  }
}

async function boot() {
  let me: Me | null = null;
  try {
    me = await api.me();
  } catch {
    me = null;
  }
  if (!me) {
    await showLogin();
    me = await api.me();
  }
  let avatar = me.avatar;
  // 后端 put_avatar 规范化掉样式字段 → 本机 localStorage 回灌样式（同色才用）。
  if (avatar && avatar.kind === "modular") {
    avatar = loadModularLocal(avatar);
  }
  if (!avatar) {
    avatar = await showAvatarCreate(null);
  }
  startGame(me, avatar);
}

void boot();
