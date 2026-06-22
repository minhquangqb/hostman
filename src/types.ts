// Khop voi struct trong src-tauri/src/models.rs

export interface Host {
  id: string;
  name: string;
  domain: string;
  target: string;
  https: boolean;
  enabled: boolean;
}

export interface Config {
  defaultTld: string;
  hosts: Host[];
}

export interface CaddyStatus {
  running: boolean;
  binary: string | null;
}

export interface GitStatus {
  is_repo: boolean;
  dirty: boolean;
  ahead: number;
  behind: number;
  remote: string | null;
}

export function emptyHost(): Host {
  return {
    id: "",
    name: "",
    domain: "",
    target: "localhost:3000",
    https: true,
    enabled: true,
  };
}
