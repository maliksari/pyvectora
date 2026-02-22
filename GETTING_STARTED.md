# PyVectora Getting Started

Bu dokuman, PyVectora ile sifirdan servis gelistirme ve mevcut projeye entegrasyon adimlarini profesyonel bir framework kullanim kilavuzu formatinda anlatir.

## 1. PyVectora Nedir

PyVectora, Python gelistirici ergonomisini Rust runtime performansi ile birlestiren hibrit bir framework yapisidir.

- HTTP sunucu, middleware zinciri ve kritik request path Rust tarafinda calisir.
- Uygulama modeli Python tarafinda `App`, `Controller`, `Contract`, `Guard`, `Provider` abstractions ile kurulur.
- Amac: Python tarafinda hizli gelistirme, Rust tarafinda dusuk gecikme ve daha iyi throughput.

## 2. Mimari Ozet

Monorepo katmanlari:

- `rust-core/pyvectora-core`: cekirdek runtime (routing, request pipeline, middleware, server state).
- `rust-core/pyvectora-bindings`: Python baglantisi (PyO3 bridge).
- `python-framework/pyvectora`: framework API katmani (`App`, DI, auth, controller, response, test client).
- `examples`: ornek uygulamalar.

## 3. Gereksinimler

- Python `3.10+` (onerilen: 3.12)
- Rust `1.70+` (rustup ile)
- `pip` ve `venv`
- `maturin` (native module build icin)

Kontrol:

```bash
python --version
rustc --version
cargo --version
```

## 4. Kaynaktan Kurulum (Development)

Proje kokunde:

```bash
cd pyvectora
python -m venv .venv
source .venv/bin/activate
pip install --upgrade pip
pip install maturin
```

Native modulu derle ve ortama bagla:

```bash
cd pyvectora/python-framework
maturin develop
```

Python framework katmanini editable modda kur:

```bash
pip install -e .
```

Dogrulama:

```bash
python -c "import pyvectora; print(pyvectora.__version__)"
```

## 5. Ilk Uygulama

`main.py`:

```python
from pyvectora import App, Response

app = App(host="127.0.0.1", port=8000)

@app.get("/")
def index(request):
    return Response.json({"service": "pyvectora", "status": "ok"})

if __name__ == "__main__":
    app.serve()
```

Calistirma:

```bash
python main.py
```

Test:

```bash
curl http://127.0.0.1:8000/
```

## 6. Controller-Tabanli Gelistirme

`controllers/user_controller.py`:

```python
from pyvectora import Controller, get, post, Response

USERS = {1: {"id": 1, "name": "Ada"}}

@Controller("/users")
class UserController:
    @get("/")
    def list_users(self):
        return Response.json({"items": list(USERS.values())})

    @get("/{id:int}")
    def get_user(self, id: int):
        user = USERS.get(id)
        if not user:
            return Response.json({"error": "not_found"}, status=404)
        return Response.json(user)

    @post("/")
    def create_user(self, request):
        payload = request.json()
        new_id = max(USERS.keys(), default=0) + 1
        user = {"id": new_id, "name": payload.get("name", "unknown")}
        USERS[new_id] = user
        return Response.json(user, status=201)
```

`main.py`:

```python
from pyvectora import App
from controllers.user_controller import UserController

app = App()
app.register_controller(UserController)

if __name__ == "__main__":
    app.serve()
```

## 7. Request Body Dogrulama (Contract)

`contracts.py`:

```python
from dataclasses import dataclass
from pyvectora import Contract

@dataclass
class CreateUser(Contract):
    name: str
    email: str
```

Controller icinde tipli kullanim:

```python
from pyvectora import Controller, post, Response
from contracts import CreateUser

@Controller("/users")
class UserController:
    @post("/")
    def create_user(self, payload: CreateUser):
        return Response.json({"name": payload.name, "email": payload.email}, status=201)
```

Not: `Contract` siniflari `@dataclass` ile tanimlanmalidir.

## 8. Dependency Injection (Provider)

`providers.py`:

```python
from pyvectora import Provider

class RequestIdProvider(Provider):
    async def provide(self, request):
        return request.headers.get("x-request-id", "generated-request-id")
```

Kullanim:

```python
from pyvectora import App, Controller, get, Response
from providers import RequestIdProvider

app = App()
app.register_provider(str, RequestIdProvider)

@Controller("/meta")
class MetaController:
    @get("/request-id")
    def request_id(self, request_id: str):
        return Response.json({"request_id": request_id})

app.register_controller(MetaController)
```

## 9. Auth ve Guard Katmani

JWT secret tanimla:

```python
app.set_jwt_secret("replace-with-strong-secret")
```

Route koruma:

```python
from pyvectora import Controller, get
from pyvectora.auth import Protected, CurrentUser

@Controller("/account")
class AccountController:
    @get("/me")
    @Protected()
    def me(self, user: CurrentUser):
        return {"sub": user.get("sub"), "role": user.get("role")}
```

## 10. Middleware ve Guvenlik Ayarlari

Rust middleware katmanini app seviyesinde ac:

```python
app.enable_logging(log_headers=False)
app.enable_timing()
app.enable_cors(
    allow_origin="https://your-frontend.example.com",
    allow_methods="GET, POST, PUT, DELETE, PATCH, OPTIONS",
    allow_headers="Content-Type, Authorization",
)
app.enable_rate_limit(capacity=200, refill_per_sec=100)
app.set_body_limit(1024 * 1024)  # 1 MB
```

Python middleware ornegi:

```python
class CorrelationIdMiddleware:
    def before_request(self, request):
        return None

    def after_response(self, request, response):
        response.with_header("X-Service", "pyvectora")
        return response

app.use_middleware(CorrelationIdMiddleware())
```

## 11. Operasyonel Endpointler

PyVectora otomatik olarak:

- `GET /docs` (Swagger UI)
- `GET /openapi.json` (OpenAPI)
- `GET /health` (uygulama saglik bilgisi)

endpointlerini uretebilir.

## 12. Test Stratejisi

`tests/test_users.py`:

```python
from pyvectora import App, Controller, get, Response

@Controller("/ping")
class PingController:
    @get("/")
    def ping(self):
        return Response.json({"ok": True})


def test_ping():
    app = App()
    app.register_controller(PingController)
    client = app.test_client()

    resp = client.get("/ping/")
    assert resp.status == 200
    assert resp.body == '{"ok": true}'
```

Calistirma:

```bash
pytest -q
```

## 13. Mevcut Projeye Entegrasyon

En az etkili entegrasyon plani:

1. Yeni bir `service boundary` secin (ornek: `/api/v1/catalog`).
2. Bu boundary icin ayri `Controller` paketleri olusturun.
3. Ortak servisler icin `Provider` tanimlayin.
4. Input dogrulamayi `Contract` ile zorunlu hale getirin.
5. `app.enable_rate_limit` ve `app.set_body_limit` ile guvenlik baseline tanimlayin.
6. `app.test_client()` ile regressions icin API testleri ekleyin.

## 14. Release ve Build Akisi

Yerel development build:

```bash
cd /Users/malik/Documents/pyvectora/python-framework
maturin develop
```

Dagitim (wheel) build:

```bash
cd /Users/malik/Documents/pyvectora/python-framework
maturin build --release
```

Cikti wheel dosyalari `target/wheels/` altinda uretilir.

## 15. Onerilen Uygulama Standartlari

- Controller methodlari yalnizca request orchestration yapmali.
- Is kurallari service katmanina alinmali.
- Tum dis API inputlari `Contract` ile validate edilmeli.
- Production ortaminda rate-limit, body-limit ve CORS explicit tanimlanmali.
- En azindan happy-path ve hata-path testleri zorunlu olmali.

## 16. Ilgili Dokumanlar

- API referansi: `/Users/malik/Documents/pyvectora/docs/API_REFERENCE.md`
- Faz ilerlemesi: `/Users/malik/Documents/pyvectora/docs/PHASE_STATUS.md`
- Ornek uygulamalar: `/Users/malik/Documents/pyvectora/examples`
