// ui/ — DOM 登录/注册界面
import { api } from "../net/api";

export function showLogin(): Promise<void> {
  return new Promise((resolve) => {
    const ui = document.getElementById("ui")!;
    ui.innerHTML = `
      <div class="overlay"><div class="panel">
        <h1>HOUSE OF IMBIBE</h1>
        <h2>一间像素小酒吧</h2>
        <label>名字</label><input id="li-user" maxlength="20" autocomplete="username" />
        <label>暗号</label><input id="li-pass" type="password" maxlength="128" autocomplete="current-password" />
        <div class="err" id="li-err"></div>
        <button class="btn" id="li-go">进 门</button>
        <button class="btn ghost" id="li-toggle">第一次来？登记个名字</button>
      </div></div>`;

    let mode: "login" | "register" = "login";
    const $ = (id: string) => document.getElementById(id)!;
    const err = (m: string) => ($("li-err").textContent = m);

    $("li-toggle").onclick = () => {
      mode = mode === "login" ? "register" : "login";
      $("li-go").textContent = mode === "login" ? "进 门" : "登 记 并 进 门";
      $("li-toggle").textContent = mode === "login" ? "第一次来？登记个名字" : "已有名字？直接进门";
      err("");
    };

    const submit = async () => {
      const username = ($("li-user") as HTMLInputElement).value.trim();
      const password = ($("li-pass") as HTMLInputElement).value;
      err("");
      try {
        if (mode === "login") await api.login(username, password);
        else await api.register(username, password);
        ui.innerHTML = "";
        resolve();
      } catch (e) {
        err((e as Error).message);
      }
    };
    $("li-go").onclick = submit;
    ui.querySelectorAll("input").forEach((i) =>
      i.addEventListener("keydown", (e) => e.key === "Enter" && submit()),
    );
    ($("li-user") as HTMLInputElement).focus();
  });
}
