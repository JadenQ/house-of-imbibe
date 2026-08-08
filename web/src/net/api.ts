// net/ — REST 客户端。不 import phaser（分层约束，见 docs/development-plan.md §2.3）

export interface Me {
  id: number;
  username: string;
  is_admin: boolean;
  avatar: AvatarData | null;
}

/** 模块化配色形象 */
export interface ModularAvatar {
  kind: "modular";
  skin: string;
  hair: string;
  shirt: string;
  pants: string;
}

/** AI 生成的 4 方向形象 */
export interface GeneratedAvatar {
  kind: "generated";
  character_id: string;
  rotations: { direction: string; url: string }[];
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
  status: "processing" | "completed" | "failed";
  error?: string;
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
  pollAvatarJob: (jobId: string) => req<AvatarJobStatus>("GET", `/api/avatar/generate/${jobId}`),
  menu: () => req<MenuPayload>("GET", "/api/menu"),
};
