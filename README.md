# PyVectora

Rust-powered Python framework for high-performance APIs, AI backends, and microservices.

PyVectora combines Python developer ergonomics with a Rust execution core. You write services with decorators, controllers, and typed contracts in Python; request routing, middleware execution, and server runtime run in Rust.

## Mission

Build a framework that gives teams:

- Python-level development speed for product iteration
- Rust-level runtime characteristics for latency, throughput, and stability
- A clean separation between framework API (Python) and execution engine (Rust)

This project is designed for teams that need more performance than pure Python frameworks without moving their full product codebase to Rust.

## Why PyVectora

- Rust runtime with Hyper + Tokio for network and request hot paths
- Python-first API for controllers, decorators, dependency injection, and contracts
- JWT auth support integrated at engine level
- Built-in middleware hooks (logging, timing, CORS, rate-limit)
- Request body size limit controls
- OpenAPI + Swagger endpoint generation
- Zero-network `test_client` for fast API tests

## Architecture At A Glance

```text
Python Layer (App, Controllers, Contracts, DI, Guards)
        |
        | PyO3 FFI Bridge
        v
Rust Core (Server, Router, Middleware, Auth, DB primitives)
        |
        v
Tokio Runtime + Hyper HTTP Stack
```

Monorepo layout:

- `rust-core/pyvectora-core`: HTTP runtime, router, middleware, request pipeline
- `rust-core/pyvectora-bindings`: Python bindings
- `rust-core/pyvectora-macros`: procedural macros
- `python-framework/pyvectora`: developer-facing framework API
- `docs`: architecture and API reference
- `examples`: sample apps and integration tests

## Quick Start (From Source)

1. Create and activate a virtual environment.
2. Build the native Rust extension for Python.
3. Install the Python framework in editable mode.
4. Run a minimal service.

```bash
cd /pyvectora
python -m venv .venv
source .venv/bin/activate
pip install --upgrade pip maturin

cd pyvectora/python-framework
maturin develop
pip install -e .
```

Minimal app:

```python
from pyvectora import App, Response

app = App()

@app.get("/")
def index(request):
    return Response.json({"message": "Hello from PyVectora"})

if __name__ == "__main__":
    app.serve()
```

Run:

```bash
python main.py
```

## Core Concepts

- `App`: application setup, middleware config, server lifecycle
- `Controller`: route grouping with shared prefix and guards
- `Contract`: typed request body validation model
- `Provider`: dependency injection abstraction for handler dependencies
- `Guard`: route-level access control
- `Response`: consistent response builder and header handling

## Current Project Status

Current line: `v0.1.x` (active development)

Implemented and usable today:

- HTTP runtime, routing, typed path params
- Controller API and decorator routing
- JWT-based protected routes
- DI provider system + guard execution
- OpenAPI schema + `/docs`, `/openapi.json`
- Built-in middleware switches: logging, timing, CORS, rate-limit
- Request body limit support
- Basic database integration primitives

In progress / hardening:

- production-grade observability and structured telemetry
- deeper middleware ergonomics for user-defined chains
- stronger lifecycle guarantees and operational hardening

## Roadmap Snapshot

- `v0.2`: production hardening, observability baseline, middleware maturity
- `v0.3`: real-time and protocol expansion (WebSocket/gRPC), policy layer
- `v1.0`: stable release with benchmark suite, compatibility guarantees, and complete docs

Full plan: see `ROADMAP.md`.

## Documentation

- Getting started: `GETTING_STARTED.md`
- API reference: `docs/API_REFERENCE.md`
- Architecture: `docs/ARCHITECTURE.md`
- Roadmap and status: `ROADMAP.md`

## Who Should Use PyVectora

- Teams moving performance-critical APIs beyond pure Python bottlenecks
- AI product teams serving latency-sensitive endpoints
- SaaS backends that need Python productivity with lower runtime overhead

## Contributing

Contributions are welcome across:

- Rust runtime performance and correctness
- Python framework DX and API quality
- Documentation and test coverage
- Examples and production deployment guides

Please open an issue for major proposals before large implementation work.

## Sponsorship

PyVectora is actively developed and open to strategic sponsorship.

Sponsorship directly funds:

- runtime performance engineering and profiling
- production hardening and reliability work
- developer tooling, docs, and long-term maintenance

If you want to sponsor development, open an issue with title `Sponsorship Inquiry` and include your target use case and required timeline.

## License

Apache License 2.0. See `LICENSE`.

---

# PyVectora (Turkce)

Yuksek performansli API'ler, AI backend'leri ve mikro servisler icin Rust destekli Python framework'u.

PyVectora, Python gelistirme ergonomisini Rust calisma cekirdegi ile birlestirir. Uygulama kodunu Python tarafinda decorator, controller ve tipli contract yapilariyla yazarsiniz; request routing, middleware calismasi ve server runtime Rust tarafinda isler.

## Amac

Takimlara su dengeyi saglayan bir framework sunmak:

- urun iterasyonu icin Python hizinda gelistirme
- gecikme, throughput ve stabilite icin Rust seviyesinde runtime davranisi
- framework API'si (Python) ile execution engine (Rust) arasinda temiz ayrim

Bu proje, tum urun kodunu Rust'a tasimadan saf Python framework'lerinin performans sinirlarini asmak isteyen takimlar icin tasarlandi.

## Neden PyVectora

- network ve request hot-path'lerinde Hyper + Tokio tabanli Rust runtime
- controller, decorator, dependency injection ve contract odakli Python-first API
- engine seviyesinde JWT auth destegi
- built-in middleware anahtarlari (logging, timing, CORS, rate-limit)
- request body size limit kontrolleri
- OpenAPI + Swagger endpoint uretimi
- hizli API testleri icin zero-network `test_client`

## Mimari Ozeti

```text
Python Katmani (App, Controllers, Contracts, DI, Guards)
        |
        | PyO3 FFI Bridge
        v
Rust Core (Server, Router, Middleware, Auth, DB primitives)
        |
        v
Tokio Runtime + Hyper HTTP Stack
```

Monorepo yapisi:

- `rust-core/pyvectora-core`: HTTP runtime, router, middleware, request pipeline
- `rust-core/pyvectora-bindings`: Python baglantilari
- `rust-core/pyvectora-macros`: procedural macro'lar
- `python-framework/pyvectora`: gelistiriciye donuk framework API'si
- `docs`: mimari ve API referansi
- `examples`: ornek uygulamalar ve entegrasyon testleri

## Hizli Baslangic (Kaynak Koddan)

1. Sanal ortami olusturun ve aktive edin.
2. Python icin native Rust extension'i derleyin.
3. Python framework katmanini editable modda kurun.
4. Minimal servisi calistirin.

```bash
cd /pyvectora
python -m venv .venv
source .venv/bin/activate
pip install --upgrade pip maturin

cd pyvectora/python-framework
maturin develop
pip install -e .
```

Minimal uygulama:

```python
from pyvectora import App, Response

app = App()

@app.get("/")
def index(request):
    return Response.json({"message": "Hello from PyVectora"})

if __name__ == "__main__":
    app.serve()
```

Calistirma:

```bash
python main.py
```

## Temel Kavramlar

- `App`: uygulama kurulumu, middleware konfigurasyonu, server lifecycle
- `Controller`: ortak prefix ve guard'lar ile route gruplama
- `Contract`: tipli request body dogrulama modeli
- `Provider`: handler bagimliliklari icin dependency injection soyutlamasi
- `Guard`: route seviyesinde erisim kontrolu
- `Response`: tutarli response builder ve header yonetimi

## Projenin Mevcut Durumu

Mevcut cizgi: `v0.1.x` (aktif gelistirme)

Bugun kullanilabilir olanlar:

- HTTP runtime, routing, tipli path parametreleri
- Controller API ve decorator routing
- JWT tabanli protected route'lar
- DI provider sistemi + guard execution
- OpenAPI schema + `/docs`, `/openapi.json`
- built-in middleware switch'leri: logging, timing, CORS, rate-limit
- request body limit destegi
- temel database entegrasyon primitifleri

Devam eden hardening alanlari:

- production-grade gozlemlenebilirlik ve structured telemetry
- user-defined middleware zinciri icin daha olgun ergonomi
- lifecycle garantilerinin ve operasyonel dayanikliligin guclendirilmesi

## Yol Haritasi Ozeti

- `v0.2`: production hardening, gozlemlenebilirlik baseline'i, middleware olgunlastirma
- `v0.3`: yetenek genisletme (WebSocket/gRPC), policy katmani
- `v1.0`: benchmark paketi, uyumluluk garantileri ve tamamlanmis dokumantasyon ile stabil surum

Detayli plan: `ROADMAP.md`.

## Dokumantasyon

- Baslangic rehberi: `GETTING_STARTED.md`
- API referansi: `docs/API_REFERENCE.md`
- Mimari: `docs/ARCHITECTURE.md`
- Yol haritasi ve durum: `ROADMAP.md`

## Kimler Icin Uygun

- performans kritik API'lerde saf Python bottleneck'ini asmak isteyen takimlar
- dusuk gecikmeli endpoint sunan AI urun takimlari
- Python verimliligini koruyup runtime overhead'i dusurmek isteyen SaaS backend ekipleri

## Katki

Katkilar su alanlarda aciktir:

- Rust runtime performansi ve dogruluk
- Python framework DX ve API kalitesi
- dokumantasyon ve test kapsami
- ornek uygulamalar ve production deployment rehberleri

Buyuk capli degisikliklerden once issue acarak teknik yonu netlestirin.

## Sponsorluk

PyVectora aktif olarak gelistiriliyor ve stratejik sponsorluga aciktir.

Sponsorluk dogrudan su calismalari fonlar:

- runtime performans muhendisligi ve profiling
- production hardening ve guvenilirlik gelistirmeleri
- gelistirici araclari, dokumantasyon ve uzun vadeli bakim

Sponsorluk icin `Sponsorship Inquiry` baslikli issue acip hedef kullanim senaryonuzu ve zaman planinizi paylasin.

## Lisans

Apache License 2.0. Ayrintilar icin `LICENSE` dosyasina bakin.
