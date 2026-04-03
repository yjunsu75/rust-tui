# tui-monitor

Rust로 작성된 터미널 기반 시스템 모니터링 도구입니다. [ratatui](https://github.com/ratatui-org/ratatui)를 사용하여 CPU, 메모리, 디스크, 네트워크 사용량을 실시간으로 시각화합니다.

## 기능

- **Overview 탭**
  - CPU: 코어별 게이지 + 전체 평균 히스토리 차트
  - Memory: RAM / Swap 게이지 + 메모리 사용률 히스토리 차트
  - Disk I/O: 읽기/쓰기 속도 히스토리 차트
  - Network I/O: 수신(↓) / 송신(↑) 속도 히스토리 차트

- **Processes 탭**
  - 실행 중인 전체 프로세스 목록 (PID, 이름, CPU%, 메모리)
  - 정렬 기준 순환: CPU → MEM → PID → NAME
  - 스크롤 지원

## 키보드 단축키

| 키 | 동작 |
|---|---|
| `Tab` / `Shift+Tab` | 탭 전환 |
| `q` / `Ctrl+C` | 종료 |
| `↑` / `k` | 프로세스 목록 위로 스크롤 |
| `↓` / `j` | 프로세스 목록 아래로 스크롤 |
| `s` | 정렬 기준 변경 |

## 기술 스택

| 크레이트 | 버전 | 용도 |
|---|---|---|
| [ratatui](https://github.com/ratatui-org/ratatui) | 0.29 | TUI 렌더링 |
| [crossterm](https://github.com/crossterm-rs/crossterm) | 0.28 | 터미널 이벤트 처리 |
| [sysinfo](https://github.com/GuillaumeGomez/sysinfo) | 0.33 | 시스템 정보 수집 |
| [tokio](https://tokio.rs) | 1 | 비동기 런타임 |

## 빌드 및 실행

```bash
# 개발 빌드
cargo run

# 릴리즈 빌드 (권장)
cargo build --release
./target/release/tui-monitor
```

## 요구 사항

- Rust 1.85 이상 (edition 2024)
- Windows / macOS / Linux

## 프로젝트 구조

```
src/
├── main.rs   # 진입점, 터미널 초기화 및 이벤트 루프
├── app.rs    # 앱 상태 및 시스템 데이터 갱신 로직
└── ui.rs     # ratatui 위젯 렌더링
```
