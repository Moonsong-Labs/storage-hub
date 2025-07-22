# StorageHub Backend Architecture

## Overview
This diagram shows the complete backend architecture as currently implemented, including the connection abstraction layer and mock implementations.

```mermaid
graph TB
    subgraph "HTTP Layer"
        API[HTTP API/Routes]
        Handlers[API Handlers]
    end

    subgraph "Service Layer"
        Services[Services]
        StorageService[Storage Service]
        PostgresService[Postgres Service]
        RpcService[RPC Service]
    end

    subgraph "Client Layer"
        PostgresClient[PostgresClient<br/>Real Implementation]
        StorageHubRpcClient[StorageHubRpcClient<br/>Real Implementation]
        MockPostgresClient[MockPostgresClient<br/>Separate Mock Implementation]
    end

    subgraph "Connection Abstraction Layer"
        DbConnectionTrait[DbConnection Trait]
        RpcConnectionTrait[RpcConnection Trait]
        
        AnyDbConnection[AnyDbConnection Enum<br/>- Real(PgConnection)<br/>- Mock(MockDbConnection)]
        AnyRpcConnection[AnyRpcConnection Enum<br/>- Real(WsConnection)<br/>- Mock(MockConnection)]
    end

    subgraph "Connection Implementations"
        PgConnection[PgConnection<br/>bb8::Pool<AsyncPgConnection>]
        MockDbConnection[MockDbConnection<br/>In-memory test data]
        
        WsConnection[WsConnection<br/>jsonrpsee WebSocket]
        MockConnection[MockConnection<br/>Mock RPC responses]
    end

    subgraph "External Systems"
        PostgresDB[(PostgreSQL<br/>Indexer Database)]
        StorageHubNode[StorageHub<br/>Parachain Node]
    end

    subgraph "Storage Layer"
        StorageWrapper[BoxedStorageWrapper]
        InMemoryStorage[InMemoryStorage]
    end

    %% HTTP to Service connections
    API --> Handlers
    Handlers --> Services
    
    %% Service dependencies
    Services --> StorageService
    Services --> PostgresService  
    Services --> RpcService
    
    %% Service to Client connections
    PostgresService --> PostgresClient
    PostgresService -.->|Tests Only| MockPostgresClient
    RpcService --> StorageHubRpcClient
    StorageService --> StorageWrapper
    
    %% Client to Connection Enum connections
    PostgresClient --> AnyDbConnection
    StorageHubRpcClient --> AnyRpcConnection
    
    %% Enum to Trait connections
    AnyDbConnection -.->|implements| DbConnectionTrait
    AnyRpcConnection -.->|implements| RpcConnectionTrait
    
    %% Enum variants
    AnyDbConnection --> PgConnection
    AnyDbConnection --> MockDbConnection
    AnyRpcConnection --> WsConnection
    AnyRpcConnection --> MockConnection
    
    %% External connections
    PgConnection --> PostgresDB
    WsConnection --> StorageHubNode
    
    %% Storage connections
    StorageWrapper --> InMemoryStorage

    %% Styling
    classDef trait fill:#f9f,stroke:#333,stroke-width:2px,stroke-dasharray: 5 5
    classDef enum fill:#bbf,stroke:#333,stroke-width:2px
    classDef mock fill:#fbb,stroke:#333,stroke-width:2px
    classDef external fill:#bfb,stroke:#333,stroke-width:2px
    
    class DbConnectionTrait,RpcConnectionTrait trait
    class AnyDbConnection,AnyRpcConnection enum
    class MockDbConnection,MockConnection,MockPostgresClient mock
    class PostgresDB,StorageHubNode external
```

## Key Architecture Components

### 1. **HTTP Layer**
- **API/Routes**: Axum-based HTTP endpoints
- **Handlers**: Request processing logic

### 2. **Service Layer**
- **Services**: Main service coordinator
- **StorageService**: File storage operations
- **PostgresService**: Database queries
- **RpcService**: Blockchain RPC calls

### 3. **Client Layer**
- **PostgresClient**: Real implementation using diesel queries
- **StorageHubRpcClient**: Real implementation using jsonrpsee
- **MockPostgresClient**: Separate mock implementation (bypasses diesel)

### 4. **Connection Abstraction Layer**
- **DbConnection Trait**: Database connection abstraction
- **RpcConnection Trait**: RPC connection abstraction
- **AnyDbConnection/AnyRpcConnection**: Enum dispatch for trait object safety

### 5. **Connection Implementations**
- **PgConnection**: Real PostgreSQL with bb8 pooling
- **MockDbConnection**: In-memory mock (but can't be used with PostgresClient due to diesel)
- **WsConnection**: WebSocket RPC connection
- **MockConnection**: Mock RPC with configurable responses

### 6. **Storage Layer**
- **BoxedStorageWrapper**: Trait object wrapper
- **InMemoryStorage**: Current implementation

## Current Issues

1. **Mock Architecture Problem**: 
   - `MockDbConnection` exists but can't be used with `PostgresClient` because it doesn't implement diesel traits
   - `MockPostgresClient` is a separate implementation, meaning production `PostgresClient` code paths are not tested
   - This violates the Stream 6 requirement of testing production code with mocks

2. **Trait Object Safety**:
   - Solved with enum dispatch pattern (AnyDbConnection, AnyRpcConnection)
   - Works well for the abstraction layer

3. **Diesel Integration**:
   - `PostgresClient` uses diesel queries directly
   - Makes it extremely difficult to mock at the connection level
   - Would require implementing complex diesel traits on MockAsyncConnection

## Intended Architecture (Stream 6 Goal)

The goal was to have:
- `PostgresClient` (production code) used in both production and tests
- `MockDbConnection` providing mock data through the same code paths
- No separate mock client implementation

But this is blocked by diesel's complex trait requirements.