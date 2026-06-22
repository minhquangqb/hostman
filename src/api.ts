import { invoke } from "@tauri-apps/api/core";
import type { CaddyStatus, Config, GitStatus, Host, ServiceStatus } from "./types";

// Config / Host CRUD
export const getConfig = () => invoke<Config>("get_config");
export const setDefaultTld = (tld: string) => invoke<Config>("set_default_tld", { tld });
export const saveHost = (host: Host) => invoke<Config>("save_host", { host });
export const deleteHost = (id: string) => invoke<Config>("delete_host", { id });
export const toggleHost = (id: string, enabled: boolean) =>
  invoke<Config>("toggle_host", { id, enabled });

// Apply
export const previewHosts = () => invoke<string>("preview_hosts");
export const previewCaddyfile = () => invoke<string>("preview_caddyfile");
export const applyAll = () => invoke<void>("apply_all");
export const applyHosts = () => invoke<void>("apply_hosts");

// Caddy
export const caddyStatus = () => invoke<CaddyStatus>("caddy_status");
export const caddyStart = () => invoke<void>("caddy_start");
export const caddyStop = () => invoke<void>("caddy_stop");
export const caddyReload = () => invoke<void>("caddy_reload");
export const caddyTrust = () => invoke<void>("caddy_trust");

// Background service (launchd)
export const serviceStatus = () => invoke<ServiceStatus>("service_status");
export const serviceInstall = () => invoke<ServiceStatus>("service_install");
export const serviceUninstall = () => invoke<ServiceStatus>("service_uninstall");

// Git
export const gitStatus = () => invoke<GitStatus>("git_status");
export const gitInit = () => invoke<GitStatus>("git_init");
export const gitSetRemote = (url: string) => invoke<GitStatus>("git_set_remote", { url });
export const gitCommit = (message: string) => invoke<GitStatus>("git_commit", { message });
export const gitPull = () => invoke<string>("git_pull");
export const gitPush = () => invoke<string>("git_push");
