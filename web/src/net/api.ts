// net/ — REST 客户端。不 import phaser（分层约束，见 docs/development-plan.md §2.3）

export interface Me {
  id: number;
  username: string;
  is_admin: boolean;
  avatar: AvatarData | null;
}

/** 模块化配色 + 样式形象（切片 2a 捏脸）。
 *  样式字段可选；缺失 → 默认 short/tshirt/pants/boots（向后兼容，不破坏现有 DEFAULT_COLORS）。
 *  样式取值见 game/character.ts 的 HAIR_STYLES / TOP_STYLES / BOTTOM_STYLES / SHOE_STYLES。 */
export interface ModularAvatar {
  kind: "modular";
  skin: string;
  hair: string;
  shirt: string;
  pants: string;
  /** 鞋色（可选；缺失 → 默认深轮廓色） */
  shoes?: string;
  /** 发型样式：short|long|bald|cap */
  hairStyle?: string;
  /** 上衣样式：tshirt|longsleeve|vest */
  topStyle?: string;
  /** 下装样式：pants|shorts|skirt */
  bottomStyle?: string;
  /** 鞋样式：boots|sneakers|sandals */
  shoeStyle?: string;
}

/** AI 生成的 4 方向形象。frames: 每方向帧 key 数组（1=静站，3=行走），经 /api/assets/{key} 取图。 */
export interface GeneratedAvatar {
  kind: "generated";
  character_id: string;
  frames: Record<"south" | "north" | "west" | "east", string[]>;
}

export type AvatarData = ModularAvatar | GeneratedAvatar;

export interface MenuItem {
  name: string;
  desc: string;
  price?: number;
}

export interface MenuPayload {
  id: string;
  sections: { title: string; items: MenuItem[] }[];
}

export interface AvatarJobStatus {
  status: "pending" | "running" | "done" | "failed";
  error?: string;
}

export interface Member {
  id: number;
  username: string;
  is_admin: boolean;
  banned: boolean;
}

/** 装饰对象 JSON 契约（广播 + 快照 + API 返回一致）。
 *  asset_id 可 null = 占位装饰（无关联资产）。 */
export interface Decoration {
  id: string;
  scene: string;
  tile_x: number;
  tile_y: number;
  asset_id: string | null;
  z_layer: number;
  placed_by: number;
}

async function req<T>(method: string, url: string, body?: unknown): Promise<T> {
  const res = await fetch(url, {
    method,
    headers: body ? { "Content-Type": "application/json" } : undefined,
    body: body ? JSON.stringify(body) : undefined,
    credentials: "same-origin",
  });
  if (!res.ok) {
    let msg = `${res.status}`;
    try {
      msg = (await res.json()).error ?? msg;
    } catch {
      /* ignore */
    }
    throw new Error(msg);
  }
  if (res.status === 204) return undefined as T;
  return res.json() as Promise<T>;
}

export const api = {
  me: () => req<Me>("GET", "/api/me"),
  register: (username: string, password: string) =>
    req<{ id: number; username: string }>("POST", "/api/register", { username, password }),
  login: (username: string, password: string) =>
    req<{ id: number; username: string }>("POST", "/api/login", { username, password }),
  logout: () => req<void>("POST", "/api/logout"),
  saveAvatar: (config: ModularAvatar) => req<void>("PUT", "/api/avatar", { config }),
  generateAvatar: async (photo: File): Promise<{ job_id: string }> => {
    const form = new FormData();
    form.append("image", photo);
    const res = await fetch("/api/avatar/generate", {
      method: "POST",
      body: form,
      credentials: "same-origin",
    });
    if (!res.ok) {
      let msg = `${res.status}`;
      try {
        msg = (await res.json()).error ?? msg;
      } catch {
        /* ignore */
      }
      throw new Error(msg);
    }
    return res.json();
  },
  generateAvatarText: (description: string) =>
    req<{ job_id: string }>("POST", "/api/avatar/generate-text", { description }),
  pollAvatarJob: (jobId: string) => req<AvatarJobStatus>("GET", `/api/avatar/generate/${jobId}`),
  menu: () => req<MenuPayload>("GET", "/api/menu"),
  admin: {
    listMembers: () => req<Member[]>("GET", "/api/admin/members"),
    promote: (id: number) => req<void>("POST", `/api/admin/members/${id}/promote`),
    demote: (id: number) => req<void>("POST", `/api/admin/members/${id}/demote`),
    ban: (id: number) => req<void>("POST", `/api/admin/members/${id}/ban`),
    unban: (id: number) => req<void>("POST", `/api/admin/members/${id}/unban`),
    listDecorations: (scene: string) =>
      req<Decoration[]>("GET", `/api/admin/decorations?scene=${encodeURIComponent(scene)}`),
    placeDecoration: (body: {
      scene: string;
      tile_x: number;
      tile_y: number;
      asset_id?: string;
      z_layer?: number;
    }) => req<Decoration>("POST", "/api/admin/decorations", body),
    removeDecoration: (id: string) => req<void>("DELETE", `/api/admin/decorations/${id}`),
  },
};
