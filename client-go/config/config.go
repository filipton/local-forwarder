package config

import (
	"encoding/json"
	"os"
	"strings"
)

type ConvertedConfig struct {
	Connector string
	Code      uint64

	ConnectorInfo ConnectorInfo
}

type Config struct {
	Connector string `json:"connector"`
	Code      uint64 `json:"code"`

	Ports []ConfigPort `json:"ports"`
}

type ConfigPort struct {
	Remote     uint16 `json:"remote"`
	Local      uint16 `json:"local"`
	Ip         string `json:"ip"`
	Type       string `json:"type"`
	TunnelType string `json:"tunnelType"`
}

type ConnectorInfo struct {
	Ports []ConnectorPort `json:"ports"`
}

type ConnectorPort struct {
	Port_remote uint16   `json:"port_remote"`
	Port_local  uint16   `json:"port_local"`
	Local_ip    string   `json:"local_ip"`
	Port_type   PortType `json:"port_type"`
	Tunnel_type PortType `json:"tunnel_type"`
}

type PortType string

const (
	TCP PortType = "Tcp"
	UDP PortType = "Udp"
)

func LoadConfig() (Config, error) {
	configJson, err := os.Open("config.json")
	if err != nil {
		return Config{}, err
	}

	defer configJson.Close()

	var config Config
	json.NewDecoder(configJson).Decode(&config)

	return config, nil
}

func (config Config) Convert() ConvertedConfig {
	convertedConfig := ConvertedConfig{
		Connector:     config.Connector,
		Code:          config.Code,
		ConnectorInfo: ConnectorInfo{},
	}

	for _, port := range config.Ports {
		convertedConfig.ConnectorInfo.Ports = append(convertedConfig.ConnectorInfo.Ports, ConnectorPort{
			Port_remote: port.Remote,
			Port_local:  port.Local,
			Local_ip:    port.Ip,
			Port_type:   convertPortType(port.Type),
			Tunnel_type: convertPortType(port.TunnelType),
		})
	}

	return convertedConfig
}

func convertPortType(portType string) PortType {
	portType = strings.ToUpper(portType)

	if portType == "UDP" {
		return UDP
	} else {
		return TCP
	}
}
