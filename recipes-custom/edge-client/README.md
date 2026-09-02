# Edge Client Service

This repository contains the Edge Client implementation designed to aggregate sensor data locally and periodically forward structured packets to a central station server.

---

## Features
* **Sensor Aggregation:** Listens for incoming TCP socket connections from local sensors (up to 5 max).
* **Upstream Forwarding:** Connects to a central station on port 9090 and transmits bundled `EdgePacket` data
* **Dynamic Configuration:** Environment variable support to dynamically set the upstream station IP without recompiling.

---

## Local Build & Usage

    ### Building Locally
    To compile the binary locally using the standard Makefile:

    ```bash
    make 
    ```
###  YOCTO Integration
    IMAGE_INSTALL:append = " edge-client"