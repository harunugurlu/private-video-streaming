# C4 System Context and Containers

This diagram shows:
- External actors
- Containers inside the system boundary
- Direct runtime relationships between them

```mermaid
flowchart TB
    user["<b>Anonymous User</b><br/>(External Actor)"]

    subgraph system["System Boundary: <b>Private Video Streaming Service</b>"]
        webapp["<b>Web App (Svelte)</b><br/>- UI states<br/>- Upload UX<br/>- Player UX"]
        api["<b>API Service (Rust)</b><br/>- Validation<br/>- Metadata/Status APIs<br/>- Orchestration<br/>- Job enqueue"]
        worker["<b>Processing Worker (Rust + ffmpeg/ffprobe)</b><br/>- Probe<br/>- Transcode<br/>- HLS packaging<br/>- Publish to public"]
        db["<b>Database (SQLite)</b><br/>- Video metadata<br/>- State transitions<br/>- Job queue"]
        storage["<b>Local File Storage</b><br/>- Source uploads<br/>- HLS tmp outputs<br/>- HLS public outputs"]
    end

    user -->|Upload, monitor progress, playback| webapp

    webapp -->|Create video, upload file, query status/playback metadata| api

    api -->|Metadata, status, share token, jobs| db
    api -->|Write uploaded source file| storage

    worker -->|Claim jobs + update status/timestamps| db
    worker -->|Read source, write/publish HLS| storage

    classDef actor fill:#edf2f7,stroke:#4a5568,color:#1a202c,stroke-width:1px;
    classDef app fill:#e6fffa,stroke:#2c7a7b,color:#1d4044,stroke-width:1px;
    classDef service fill:#ebf8ff,stroke:#2b6cb0,color:#1a365d,stroke-width:1px;
    classDef worker fill:#fffaf0,stroke:#b7791f,color:#744210,stroke-width:1px;
    classDef data fill:#f7fafc,stroke:#4a5568,color:#2d3748,stroke-width:1px;

    class user actor;
    class webapp app;
    class api service;
    class worker worker;
    class db,storage data;
```

## Notes

- Arrows represent direct runtime interactions only.
- Request/response is represented by a single request-direction arrow for readability.
- API and Worker coordinate asynchronously through the DB-backed job queue (no direct API -> Worker call).
