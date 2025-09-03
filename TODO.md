# TODO: 以 sysinfo 整合每個 Process 的 CPU / Disk / IO 統計

> 目標：在目前的連線清單中，針對 `associated_pids` 對應的每個 Process，額外顯示 CPU 使用率、磁碟讀寫速率與 I/O 統計。並提供取樣間隔、是否顯示/隱藏統計等 CLI 參數。

## 規劃與設計
- [x] 定義資料結構：新增 `ProcessStats`（含 `cpu_pct`, `read_per_sec`, `write_per_sec`, `total_read_bytes`, `total_written_bytes` 等）。
- [x] 取樣策略：CPU/IO 需要至少兩次 refresh 才能得出「速率」。
  - 第一次呼叫收集初值，等待 `--sample-interval`（預設 500–1000ms），再次 refresh 後以「差值/時間」計算速率。
  - 避免 `refresh_all()`，改用最小化 refresh（只刷新 process + disks）。
 - [ ] 平台相容：以 `sysinfo` 的可用欄位為主，若某平台不支援特定計數則以 `N/A` 顯示，保持程式不崩潰。（目前以缺值顯示 N/A；後續再補更細平台註記）
  - [x] Windows: 透過 EStats 取得每個 PID 的 TCP Rx/Tx 並彙總顯示。
  - [ ] Linux/macOS: 研究以 netlink/sock_diag 或其他方法取得 per-process Rx/Tx（目前顯示 N/A）。

## CLI 與使用者介面
- [x] 新增 CLI 參數（暫定）：
  - `--full`/`-f`：顯示 CPU/Disk/IO 欄位與每 Process 網路欄位。
  - `--sample-interval <millis>`：取樣間隔（預設 800ms）。
  - `--top <n>`（選擇性）：僅針對每列 socket 的前 n 個 PID 顯示統計，避免輸出過寬。
 - [x] 輸出欄位規劃：
  - 既有：`PROTO | LOCAL ADDRESS | REMOTE ADDRESS | STATE | PROCESS`
   - 新增：`CPU% | R/s | W/s | Rx/s | Tx/s`（必要），`R Total | W Total`（選擇性）。
  - 注意欄寬：Windows 主機名稱與路徑較長，需節流/截斷顯示。

## 實作步驟
- [x] 抽離目前 `main.rs` 中收集 socket 的邏輯至小型 helper（便於插入統計流程）。
- [x] 新增 `collect_process_stats(system: &mut System, pids: &[u32], interval: Duration) -> HashMap<u32, ProcessStats>`：
  - 以 `System::new_all()` 或 `System::new()` + 精準 refresh。
  - 第一次 refresh：`system.refresh_processes_specifics(...)`；記錄初始 `process.disk_usage()` 與 `process.cpu_usage()` 基準。
  - `sleep(interval)` 後第二次 refresh：再取值並計算每秒速率 與 CPU%。
- [x] 整合輸出：將 `pid -> ProcessStats` 映射回原本的 `process_info`，於對應列追加新欄位。
- [x] 錯誤處理：找不到 Process 或欄位不可用時，顯示 `N/A`；不讓整體流程失敗。

## 測試與品質
- [ ] 單元測試：
  - 格式化/對齊邏輯（固定輸入 -> 固定輸出寬度）。
  - `human_readable_bytes` / `human_readable_rate`（若新增）。
- [ ] 整合測試：
  - Snapshot 測 `--help` 與 `--version`。
  - 在無特權與無網路假設下，對 `--full --sample-interval 1` 做最小 smoke 測（確保不 panic）。
- [x] Lint/格式：`cargo clippy -D warnings`、`cargo fmt --all` 通過。

## 文件與維護
- [x] README：新增使用範例（含 `--full`、`--sample-interval`）。
- [x] 註解：標示平台差異與 `sysinfo` 欄位刷新要求（需要兩次 refresh 才有速率/CPU%）。
- [ ] 版本標註：於 `CHANGELOG` 或 README 中記錄新增功能。

## 風險與注意事項
- [ ] 效能：避免在所有 PID 上做過多次 refresh；僅針對目前頁面/列出的 PID 取樣。
- [ ] 欄位可用性：不同 OS 對 `disk_usage`/CPU 維度支持不一；需寬容處理。
- [ ] 權限：取 process 資訊在部分環境需要額外權限；失敗時以 `Unknown/N/A` 退場。

---

## 粗略時程建議
- 第一天：CLI 與資料收集原型（僅 CPU%）。
- 第二天：加入 Disk/IO 速率與對齊輸出。
- 第三天：測試、文件、清理、跨平台最小驗證。
