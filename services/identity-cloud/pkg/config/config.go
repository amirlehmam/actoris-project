// Package config provides configuration for IdentityCloud service
package config

import (
	"fmt"
	"os"
	"strconv"
	"time"
)

// Config holds all configuration for IdentityCloud
type Config struct {
	// Server settings
	Server ServerConfig

	// Neo4j settings
	Neo4j Neo4jConfig

	// Service settings
	Service ServiceConfig
}

// ServerConfig holds gRPC server configuration
type ServerConfig struct {
	Host            string
	Port            int
	MaxRecvMsgSize  int
	MaxSendMsgSize  int
	ShutdownTimeout time.Duration
}

// Neo4jConfig holds Neo4j connection configuration
type Neo4jConfig struct {
	URI      string
	Username string
	Password string
	Database string
}

// ServiceConfig holds service-specific settings
type ServiceConfig struct {
	// Initial HC balance for new wallets
	InitialHCBalance string
	// Wallet expiry duration
	WalletExpiryDays int
	// Enable trust inheritance for spawned agents
	EnableTrustInheritance bool
	// Default trust score for new identities
	DefaultTrustScore uint32
}

// Load loads configuration from environment variables
func Load() (*Config, error) {
	cfg := &Config{
		Server: ServerConfig{
			Host:            getEnv("IDENTITY_SERVER_HOST", "0.0.0.0"),
			Port:            getEnvInt("IDENTITY_SERVER_PORT", 50051),
			MaxRecvMsgSize:  getEnvInt("IDENTITY_MAX_RECV_MSG_SIZE", 4*1024*1024), // 4MB
			MaxSendMsgSize:  getEnvInt("IDENTITY_MAX_SEND_MSG_SIZE", 4*1024*1024), // 4MB
			ShutdownTimeout: time.Duration(getEnvInt("IDENTITY_SHUTDOWN_TIMEOUT_SECS", 30)) * time.Second,
		},
		Neo4j: Neo4jConfig{
			URI:      getEnv("NEO4J_URI", "bolt://localhost:7687"),
			Username: getEnv("NEO4J_USERNAME", "neo4j"),
			Password: getEnv("NEO4J_PASSWORD", ""),
			Database: getEnv("NEO4J_DATABASE", "neo4j"),
		},
		Service: ServiceConfig{
			InitialHCBalance:       getEnv("IDENTITY_INITIAL_HC_BALANCE", "0"),
			WalletExpiryDays:       getEnvInt("IDENTITY_WALLET_EXPIRY_DAYS", 30),
			EnableTrustInheritance: getEnvBool("IDENTITY_ENABLE_TRUST_INHERITANCE", true),
			DefaultTrustScore:      uint32(getEnvInt("IDENTITY_DEFAULT_TRUST_SCORE", 500)),
		},
	}

	// Validate required config
	if cfg.Neo4j.Password == "" {
		return nil, fmt.Errorf("NEO4J_PASSWORD environment variable is required")
	}

	return cfg, nil
}

// Address returns the server address
func (c *ServerConfig) Address() string {
	return fmt.Sprintf("%s:%d", c.Host, c.Port)
}

// getEnv gets an environment variable with a default value
func getEnv(key, defaultValue string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return defaultValue
}

// getEnvInt gets an integer environment variable with a default value
func getEnvInt(key string, defaultValue int) int {
	if value := os.Getenv(key); value != "" {
		if intVal, err := strconv.Atoi(value); err == nil {
			return intVal
		}
	}
	return defaultValue
}

// getEnvBool gets a boolean environment variable with a default value
func getEnvBool(key string, defaultValue bool) bool {
	if value := os.Getenv(key); value != "" {
		if boolVal, err := strconv.ParseBool(value); err == nil {
			return boolVal
		}
	}
	return defaultValue
}
