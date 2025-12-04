// Package main provides the entry point for IdentityCloud service
package main

import (
	"context"
	"fmt"
	"log"
	"net"
	"os"
	"os/signal"
	"syscall"

	"github.com/actoris/actoris/services/identity-cloud/internal/repository"
	"github.com/actoris/actoris/services/identity-cloud/internal/service"
	"github.com/actoris/actoris/services/identity-cloud/pkg/config"
	"google.golang.org/grpc"
	"google.golang.org/grpc/health"
	"google.golang.org/grpc/health/grpc_health_v1"
	"google.golang.org/grpc/reflection"
)

func main() {
	// Load configuration
	cfg, err := config.Load()
	if err != nil {
		log.Fatalf("Failed to load configuration: %v", err)
	}

	// Create context with cancellation
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// Initialize Neo4j repository
	repo, err := repository.NewNeo4jRepository(ctx, cfg.Neo4j.URI, cfg.Neo4j.Username, cfg.Neo4j.Password)
	if err != nil {
		log.Fatalf("Failed to connect to Neo4j: %v", err)
	}
	defer repo.Close(ctx)

	log.Println("Connected to Neo4j database")

	// Initialize database schema
	if err := initializeSchema(ctx, repo); err != nil {
		log.Printf("Warning: failed to initialize schema: %v", err)
	}

	// Initialize service
	svc := service.NewIdentityService(repo)

	// Create gRPC server
	grpcServer := grpc.NewServer(
		grpc.MaxRecvMsgSize(cfg.Server.MaxRecvMsgSize),
		grpc.MaxSendMsgSize(cfg.Server.MaxSendMsgSize),
	)

	// Register health service
	healthServer := health.NewServer()
	grpc_health_v1.RegisterHealthServer(grpcServer, healthServer)
	healthServer.SetServingStatus("", grpc_health_v1.HealthCheckResponse_SERVING)
	healthServer.SetServingStatus("identity", grpc_health_v1.HealthCheckResponse_SERVING)

	// Enable reflection for development
	reflection.Register(grpcServer)

	// Note: The IdentityServer would be registered here once proto generation is set up
	// identityServer := grpcimpl.NewIdentityServer(svc)
	// pb.RegisterIdentityServiceServer(grpcServer, identityServer)
	_ = svc // Use service (will be used when proto registration is added)

	// Create listener
	listener, err := net.Listen("tcp", cfg.Server.Address())
	if err != nil {
		log.Fatalf("Failed to listen on %s: %v", cfg.Server.Address(), err)
	}

	// Start server in goroutine
	errCh := make(chan error, 1)
	go func() {
		log.Printf("IdentityCloud gRPC server starting on %s", cfg.Server.Address())
		if err := grpcServer.Serve(listener); err != nil {
			errCh <- fmt.Errorf("failed to serve: %w", err)
		}
	}()

	// Wait for shutdown signal
	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGINT, syscall.SIGTERM)

	select {
	case err := <-errCh:
		log.Printf("Server error: %v", err)
	case sig := <-sigCh:
		log.Printf("Received signal %v, shutting down...", sig)
	}

	// Graceful shutdown
	shutdownCtx, shutdownCancel := context.WithTimeout(context.Background(), cfg.Server.ShutdownTimeout)
	defer shutdownCancel()

	// Set health to not serving
	healthServer.SetServingStatus("", grpc_health_v1.HealthCheckResponse_NOT_SERVING)
	healthServer.SetServingStatus("identity", grpc_health_v1.HealthCheckResponse_NOT_SERVING)

	// Graceful stop
	stopped := make(chan struct{})
	go func() {
		grpcServer.GracefulStop()
		close(stopped)
	}()

	select {
	case <-stopped:
		log.Println("Server stopped gracefully")
	case <-shutdownCtx.Done():
		log.Println("Shutdown timeout exceeded, forcing stop")
		grpcServer.Stop()
	}

	log.Println("IdentityCloud shutdown complete")
}

// initializeSchema sets up Neo4j constraints and indexes
func initializeSchema(ctx context.Context, repo *repository.Neo4jRepository) error {
	// Create constraints for UnifiedID nodes
	constraints := []string{
		"CREATE CONSTRAINT unified_id_did IF NOT EXISTS FOR (u:UnifiedID) REQUIRE u.did IS UNIQUE",
		"CREATE INDEX unified_id_entity_type IF NOT EXISTS FOR (u:UnifiedID) ON (u.entity_type)",
		"CREATE INDEX unified_id_parent IF NOT EXISTS FOR (u:UnifiedID) ON (u.parent_did)",
		"CREATE INDEX trust_score_score IF NOT EXISTS FOR (t:TrustScore) ON (t.score)",
		"CREATE INDEX wallet_owner IF NOT EXISTS FOR (w:HCWallet) ON (w.owner_did)",
		"CREATE INDEX wallet_expires IF NOT EXISTS FOR (w:HCWallet) ON (w.expires_at)",
	}

	for _, constraint := range constraints {
		if err := repo.ExecuteWrite(ctx, constraint, nil); err != nil {
			// Log but continue - constraint may already exist
			log.Printf("Warning: failed to create constraint: %v", err)
		}
	}

	return nil
}
