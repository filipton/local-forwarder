package main

import (
	"encoding/json"
	"fmt"
	"io"
	"local-forwarder/butils"
	"local-forwarder/config"
	"net"
)

func main() {
	c, err := config.LoadConfig()
	if err != nil {
		fmt.Println("Config error: ", err)
		return
	}
	convertedConfig := c.Convert()

	conn, err := net.Dial("tcp", convertedConfig.Connector)
	if err != nil {
		fmt.Println(err)
		return
	}

	codeBytes := butils.FromUint64(convertedConfig.Code)
	portBytes := butils.FromUint16(0)

	tmpBytes := append(portBytes, codeBytes...)
	conn.Write(tmpBytes)

	bytesConnector, err := json.Marshal(convertedConfig.ConnectorInfo)
	if err != nil {
		fmt.Println("Json stringify error: ", err)
	}

	lenBytes := butils.FromUint16(uint16(len(bytesConnector)))
	conn.Write(lenBytes)
	conn.Write(bytesConnector)

	for {
		buf := make([]byte, 1024)
		_, err = conn.Read(buf)
		if err != nil {
			fmt.Println(err)
			return
		}

		go spawnProxy(convertedConfig, buf)
	}
}

func spawnProxy(c config.ConvertedConfig, buf []byte) {
	port := butils.ToUint16(buf[:2])
	localPort := config.ConnectorPort{}

	for _, port_entry := range c.ConnectorInfo.Ports {
		if port_entry.Port_remote == port {
			localPort = port_entry
		}
	}

	tunnel, err := spawnTunnel(port, c.Code, c.Connector)
	if err != nil {
		fmt.Println("Tunnel error: ", err)
		return
	}

	local, err := net.Dial("tcp", fmt.Sprintf("%s:%d", localPort.Local_ip, localPort.Port_local))
	if err != nil {
		fmt.Println("Local error: ", err)
		return
	}

	copyBidirectional(tunnel, local)
}

func spawnTunnel(port uint16, code uint64, tunnelIp string) (net.Conn, error) {
	conn, err := net.Dial("tcp", tunnelIp)
	if err != nil {
		return nil, err
	}

	portBytes := butils.FromUint16(port)
	codeBytes := butils.FromUint64(code)

	tmpBytes := append(portBytes, codeBytes...)
	conn.Write(tmpBytes)
	return conn, nil
}

func copyBidirectional(a, b net.Conn) {
	defer func() {
		a.Close()
		b.Close()
	}()

	done := make(chan struct{})

	go func() {
		io.Copy(a, b)
		done <- struct{}{}
	}()

	go func() {
		io.Copy(b, a)
		done <- struct{}{}
	}()

	<-done
}
