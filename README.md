# Local Forwarder
Simple and easy to use port forwarder.

## Use case
If you want to forward and port but your home router (or your ISP) 
doesn't allow that you can use this to forward local port to remote server.

## Manual Setup
1. Download server binary on your remote machine that allows port forwarding
2. Run it and copy generated code (if you want to change the code edit /etc/local-forwarder/config.json file)
3. Download client binary on your local machine
4. Configure it [configuration example](#client-configuration)

### Server Configuration
```json
{
  "code": 1234567890,
  "port": 1337
}
```

|          | Explanation                                            |
|----------|--------------------------------------------------------|
| **code** | connector code (must be the same in client to connect) |
| **port** | connector port                                         |

### Client Configuration
```json
{
  "connector": "remote_ip:1337",
  "code": 1234567890,
  "ports": [
    {
      "remote": 80,
      "local": 80,
      "ip": "127.0.0.1",
      "type": "TCP",
      "tunnelType": "UDP"
    }
  ]
}
```

|               | Explanation                           |
|---------------|---------------------------------------|
| **connector** | address ip with port to the connector |
| **code**      | connector code                        |
| **ports**     | list of forwarded ports               |

#### Port entry
|                | Explanation                   |
|----------------|-------------------------------|
| **remote**     | port on remote server         |
| **local**      | port on "local" machine       |
| **ip**         | ip to "local" machine         |
| **type**       | port type (TCP \| UDP)        |
| **tunnelType** | tunnel port type (TCP \| UDP) |

> **Warning**
> Tunnel type almost always should be TCP, because UDP is highly unstable and slow.
> Another flow of UDP tunnel type is that golang client doesn't support it yet.

## Docker Setup
You can also use docker images to easily create tunnels.

Simple docker-compose file can be found [here](./docker/docker-compose.yml)

### Env config
As you can see for easier configuration in docker you can use environment variables

Client "ports notation" is similar to docker's "ports notation".

Example: 192.168.1.38:8080:80/tcp

|                  | Explanation                                    |
|------------------|------------------------------------------------|
| **192.168.1.38** | ip to your service (even not in local network) |
| **8080**         | port on remote machine (lf-server)             |
| **80**           | port on local machine (lf-client)              |
| **tcp**          | port type                                      |

> **Warning**
> Each of your ports specified in env **must have different key** (e.g. LF_PORT1, LF_PORT2...)
