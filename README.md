# Pipe Client Server
Клиент-сервер на Rust для управления задачами через JSON по именованным каналам (FIFO).  

## Описание  
Приложение реализует простой менеджер задач с коммуникацией через Unix FIFO (named pipes).  
Сервер хранит до 5 задач в памяти и обрабатывает запросы на добавление, получение и удаление.  

┌─────────────┐         ┌─────────────┐  
│   Client    │ ─────►  │   Server    │  
│  (writer)   │  pipe   │  (reader)   │  
│             │ .request│             │  
└─────────────┘         └──────┬──────┘  
                               │  
┌─────────────┐         ┌──────▼──────┐  
│   Client    │ ◄─────  │   Server    │  
│  (reader)   │  pipe   │  (writer)   │  
│             │ .response.{pid}       │  
└─────────────┘         └─────────────┘  

{pipe-name}.request — канал для запросов (клиент → сервер)  

{pipe-name}.response.{pid} — уникальный канал для ответов (сервер → клиент)

## Запуск

```bash
# Терминал 1: запустить сервер
cargo run --bin server -- --pipe-name="./pipe"
# или из release-сборки:
./target/release/server --pipe-name="./pipe"

# Терминал 2: запустить клиента
# Добавить задачу:
cargo run --bin client -- --pipe-name="./pipe" --request=add --parameter="Купить хлеб"

# Получить задачу по ID:
cargo run --bin client -- --pipe-name="./pipe" --request=get --parameter=1

# Удалить задачу по ID:
cargo run --bin client -- --pipe-name="./pipe" --request=delete --parameter=1
```

**После завершения работы сервера удалите оставшиеся FIFO-файлы:**  

```bash
rm -f ./pipe.request ./pipe.response.*  
```

### Author  
Антон Евгеньев, red1house1 (tg: @tdutanton)
2026
