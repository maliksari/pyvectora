# PyVectora Roadmap

This roadmap defines where PyVectora is now, what is being built next, and the target path to a production-grade `v1.0` release.

## Product Direction

PyVectora aims to be a high-performance Python framework with a Rust execution core.

Guiding principles:

- Python for application composition and fast iteration
- Rust for request-path performance and runtime stability
- Clear framework/engine boundary (no framework cloning, no API mimicry)
- Incremental hardening with measurable production criteria

## Current State (As Of 2026-02-22)

Release line: `v0.1.x`

Status summary:

- foundation and core runtime are in place
- main framework abstractions are available and usable
- production hardening is partially complete

## Completed Milestones

### M0: Foundation

- Rust workspace structure and crate separation
- Python packaging with maturin bridge
- baseline lint/test setup

### M1: Runtime Core

- Hyper + Tokio HTTP server
- route registration and dispatch
- graceful shutdown and connection drain behavior

### M2: Routing + Request Pipeline

- typed path params
- method routing (GET/POST/PUT/PATCH/DELETE/OPTIONS/HEAD)
- request/response bridge between Rust and Python

### M3: Framework API

- `App`, `Controller`, route decorators
- `Response` helpers
- basic request object interface

### M4: Security + Validation

- JWT-protected route support
- guard pattern
- contract-based body validation

### M5: Extensibility + DX

- provider-based dependency injection
- OpenAPI generation and docs endpoint
- zero-network test client support

### M6: Engine Controls

- middleware toggles (logging, timing, CORS, rate-limit)
- request body size limit controls

## Active Workstreams

### P0 (Highest Priority, Production Blocking)

1. Deterministic lifecycle and shutdown hardening
- Goal: predictable startup/ready/shutdown behavior under load
- Minimum viable plan: standardize lifecycle hooks and ordering
- Minimum viable plan: enforce drain timeout policy and explicit failure modes
- Minimum viable plan: add integration tests for shutdown edge cases

2. Observability baseline for real operations
- Goal: actionable logs/traces in production incidents
- Minimum viable plan: structured JSON logging schema
- Minimum viable plan: request-id propagation and correlation
- Minimum viable plan: baseline tracing spans for request pipeline

3. Middleware maturity for real services
- Goal: stable composition of engine and Python middleware
- Minimum viable plan: formal middleware order and execution contract
- Minimum viable plan: error isolation rules for middleware failures
- Minimum viable plan: tests for short-circuit, header mutation, and ordering

4. Security and edge protections
- Goal: safe defaults for internet-facing APIs
- Minimum viable plan: finalize body-limit behavior and error contract
- Minimum viable plan: harden rate-limit behavior and identity strategy
- Minimum viable plan: document authn/authz separation boundaries

### P1 (Production Grade)

1. Binary distribution and compatibility
- Minimum viable plan: publish-ready wheel matrix strategy (platform and Python version)
- Minimum viable plan: compatibility test matrix and ABI checks

2. Developer experience hardening
- Minimum viable plan: precise typing coverage and stubs
- Minimum viable plan: stronger error messages for DI/contract failures
- Minimum viable plan: tighter CLI run experience and diagnostics

3. Database integration maturation
- Minimum viable plan: strengthen current DB primitives and connection handling
- Minimum viable plan: define extension points for future dedicated ORM project
- Scope note: keep ORM migrations out of this repository for now

### P2 (Strategic Expansion)

1. Protocol expansion
- Candidate deliverable: WebSocket support
- Candidate deliverable: gRPC support

2. Policy and authorization layer
- Candidate deliverable: pluggable policy evaluation for role/attribute-based authorization

3. Performance engineering program
- Candidate deliverable: benchmark suite and regression budget tracking
- Candidate deliverable: hotspot profiling and GIL-boundary optimization pass

## Planned Releases

### v0.2 Target

Focus: production hardening and operations baseline.

Expected outcomes:

- stronger lifecycle and shutdown behavior
- structured logging and better diagnostics
- middleware contract stabilization
- hardened rate-limit/body-limit behavior

### v0.3 Target

Focus: capability expansion and ecosystem fit.

Expected outcomes:

- protocol expansion groundwork (WebSocket/gRPC)
- authorization policy primitives
- improved platform integration patterns

### v1.0 Target

Focus: stable, supportable framework release.

Exit criteria:

- stable API contract for core framework abstractions
- documented production deployment guidance
- benchmark suite with repeatable methodology
- release engineering and compatibility guarantees

## Explicitly Out Of Scope (Current Repo)

The following are intentionally deferred to separate efforts:

- workflow engine implementation
- full ORM system and migration framework

This repository keeps focus on framework + runtime core.

## How Progress Is Tracked

This file is the single public source for roadmap status.

Progress handling approach:

- milestone completion is reflected directly under `Completed Milestones`
- active engineering priorities are maintained under `P0/P1/P2`
- release targets (`v0.2`, `v0.3`, `v1.0`) are updated here as scope changes
- out-of-scope boundaries are explicitly documented to prevent roadmap drift

## Sponsor Alignment

For sponsors and design partners, priority investment areas are:

- P0 reliability and observability hardening
- platform-quality release engineering
- performance profiling and optimization program

If your team needs a specific milestone accelerated, open an issue with title `Roadmap Partnership` and include:

- workload profile
- target SLA/latency expectations
- timeline and deployment constraints

---

# PyVectora Roadmap (Turkce)

Bu yol haritasi, PyVectora'nin bugunku durumunu, sonraki gelistirme alanlarini ve production-grade `v1.0` hedefine giden yolu tanimlar.

## Urun Yonu

PyVectora, Rust calisma cekirdegi uzerine kurulu yuksek performansli bir Python framework olmayi hedefler.

Yon verici prensipler:

- uygulama kompozisyonu ve hizli iterasyon icin Python
- request-path performansi ve runtime stabilitesi icin Rust
- net framework/engine siniri (framework klonlama yok, birebir API taklidi yok)
- olculebilir production kriterleriyle asamali hardening

## Mevcut Durum (2026-02-22)

Surum cizgisi: `v0.1.x`

Durum ozeti:

- temel mimari ve core runtime hazir
- ana framework soyutlamalari kullanilabilir
- production hardening calismalari kismen tamamlandi

## Tamamlanan Asamalar

### M0: Foundation

- Rust workspace yapisi ve crate ayrimi
- maturin ile Python paketleme koprusu
- temel lint/test duzeni

### M1: Runtime Core

- Hyper + Tokio HTTP server
- route kaydi ve dispatch
- graceful shutdown ve baglanti drain davranisi

### M2: Routing + Request Pipeline

- tipli path parametreleri
- method routing (GET/POST/PUT/PATCH/DELETE/OPTIONS/HEAD)
- Rust-Python request/response koprusu

### M3: Framework API

- `App`, `Controller`, route decorator seti
- `Response` yardimcilari
- temel request arayuzu

### M4: Security + Validation

- JWT-protected route destegi
- guard modeli
- contract tabanli body dogrulama

### M5: Extensibility + DX

- provider tabanli dependency injection
- OpenAPI uretimi ve docs endpoint'i
- zero-network test client destegi

### M6: Engine Controls

- middleware anahtarlari (logging, timing, CORS, rate-limit)
- request body size limit kontrolleri

## Aktif Is Kalemleri

### P0 (En Yuksek Oncelik, Production Blocking)

1. Deterministik lifecycle ve shutdown hardening
- Hedef: yuk altinda ongorulebilir startup/ready/shutdown davranisi
- Minimum uygulanabilir plan: lifecycle hook sirasi ve sozlesmesini standardize etmek
- Minimum uygulanabilir plan: drain timeout politikasi ve acik failure modlari
- Minimum uygulanabilir plan: shutdown edge-case'leri icin entegrasyon testleri

2. Gercek operasyonlar icin gozlemlenebilirlik baseline'i
- Hedef: production incident'lerde eyleme donuk log/trace kalitesi
- Minimum uygulanabilir plan: structured JSON logging semasi
- Minimum uygulanabilir plan: request-id propagation ve korelasyon
- Minimum uygulanabilir plan: request pipeline icin temel tracing span'lari

3. Gercek servisler icin middleware olgunlastirma
- Hedef: engine ve Python middleware bileşiminin stabil hale gelmesi
- Minimum uygulanabilir plan: middleware sira ve calisma sozlesmesinin netlestirilmesi
- Minimum uygulanabilir plan: middleware hata durumlarinda izolasyon kurallari
- Minimum uygulanabilir plan: short-circuit, header mutasyonu ve sira testleri

4. Guvenlik ve edge korumalari
- Hedef: internet'e acik API'ler icin guvenli varsayilanlar
- Minimum uygulanabilir plan: body-limit davranisi ve hata sozlesmesinin netlestirilmesi
- Minimum uygulanabilir plan: rate-limit stratejisi ve identity modelinin sertlestirilmesi
- Minimum uygulanabilir plan: authn/authz ayrim sinirlarinin dokumante edilmesi

### P1 (Production Grade)

1. Binary dagitim ve uyumluluk
- Minimum uygulanabilir plan: platform ve Python surumleri icin wheel matrix stratejisi
- Minimum uygulanabilir plan: uyumluluk test matrix'i ve ABI kontrolleri

2. Gelistirici deneyimi hardening
- Minimum uygulanabilir plan: tip kapsami ve stub iyilestirmeleri
- Minimum uygulanabilir plan: DI/contract hatalari icin daha net hata mesajlari
- Minimum uygulanabilir plan: CLI run deneyimi ve tani araclarinin guclendirilmesi

3. Database entegrasyonu olgunlastirma
- Minimum uygulanabilir plan: mevcut DB primitifleri ve connection handling'in iyilestirilmesi
- Minimum uygulanabilir plan: ayri ORM projesi icin extension point'lerin tanimlanmasi
- Kapsam notu: ORM migration bu repository kapsaminda tutulmayacak

### P2 (Stratejik Genisleme)

1. Protokol genisletme
- Aday teslimat: WebSocket destegi
- Aday teslimat: gRPC destegi

2. Policy ve yetkilendirme katmani
- Aday teslimat: role/attribute bazli pluggable policy degerlendirme

3. Performans muhendisligi programi
- Aday teslimat: benchmark suiti ve regresyon butcesi takibi
- Aday teslimat: hotspot profiling ve GIL-boundary optimizasyonu

## Planlanan Surumler

### v0.2 Hedefi

Odak: production hardening ve operasyonel baseline.

Beklenen ciktilar:

- daha guclu lifecycle ve shutdown davranisi
- structured logging ve daha iyi tanilama
- middleware sozlesmesinin stabilizasyonu
- rate-limit/body-limit davranislarinin sertlestirilmesi

### v0.3 Hedefi

Odak: yetenek genisletme ve ekosistem uyumu.

Beklenen ciktilar:

- protokol genisletme zemini (WebSocket/gRPC)
- policy/yetkilendirme primitifleri
- platform entegrasyon kaliplari

### v1.0 Hedefi

Odak: stabil ve desteklenebilir framework surumu.

Cikis kriterleri:

- temel framework soyutlamalari icin stabil API sozlesmesi
- production deployment rehberlerinin tamamlanmasi
- tekrarlanabilir metodolojiye sahip benchmark suiti
- release engineering ve uyumluluk garantileri

## Bilerek Kapsam Disi Birakilanlar (Bu Repo)

Asagidaki konular ayri calismalara ertelenmistir:

- workflow engine implementasyonu
- tam kapsamli ORM sistemi ve migration framework'u

Bu repository framework + runtime core odagina sadik kalir.

## Ilerleme Takibi

Bu dosya, roadmap durumunun tek kamusal kaynagidir.

Takip yaklasimi:

- milestone tamamlanma bilgisi dogrudan `Completed Milestones` altinda tutulur
- aktif muhendislik oncelikleri `P0/P1/P2` altinda yonetilir
- surum hedefleri (`v0.2`, `v0.3`, `v1.0`) kapsam degistikce burada guncellenir
- roadmap sapmasini engellemek icin kapsam disi sinirlar acik sekilde yazilir

## Sponsor Uyum Alanlari

Sponsorlar ve design partner'lar icin oncelikli yatirim alanlari:

- P0 guvenilirlik ve gozlemlenebilirlik hardening
- platform kalitesinde release engineering
- performans profiling ve optimizasyon programi

Belirli bir milestone'u hizlandirmak istiyorsaniz `Roadmap Partnership` baslikli issue acip su bilgileri ekleyin:

- is yuku profili
- hedef SLA/gecikme beklentileri
- zaman plani ve deployment kisitlari
