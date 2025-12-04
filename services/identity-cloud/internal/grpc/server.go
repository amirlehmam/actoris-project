// Package grpc provides the gRPC server implementation for IdentityCloud
package grpc

import (
	"context"
	"crypto/ed25519"
	"encoding/base64"

	"github.com/actoris/actoris/services/identity-cloud/internal/domain"
	"github.com/actoris/actoris/services/identity-cloud/internal/service"
	"github.com/shopspring/decimal"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

// IdentityServer implements the IdentityService gRPC server
type IdentityServer struct {
	UnimplementedIdentityServiceServer
	svc *service.IdentityService
}

// NewIdentityServer creates a new IdentityServer
func NewIdentityServer(svc *service.IdentityService) *IdentityServer {
	return &IdentityServer{svc: svc}
}

// CreateUnifiedID creates a new UnifiedID
func (s *IdentityServer) CreateUnifiedID(ctx context.Context, req *CreateUnifiedIDRequest) (*CreateUnifiedIDResponse, error) {
	if req.EntityType == EntityType_ENTITY_TYPE_UNSPECIFIED {
		return nil, status.Error(codes.InvalidArgument, "entity_type is required")
	}

	entityType := protoToEntityType(req.EntityType)

	var parentDID *string
	if req.ParentDid != nil {
		parentDID = req.ParentDid
	}

	identity, err := s.svc.CreateIdentity(ctx, entityType, parentDID)
	if err != nil {
		return nil, status.Errorf(codes.Internal, "failed to create identity: %v", err)
	}

	// Get initial trust score
	trustScore, err := s.svc.GetTrustScore(ctx, identity.DID)
	if err != nil {
		return nil, status.Errorf(codes.Internal, "failed to get trust score: %v", err)
	}

	// Get wallet
	wallet, err := s.svc.GetHCWallet(ctx, identity.DID)
	if err != nil {
		return nil, status.Errorf(codes.Internal, "failed to get wallet: %v", err)
	}

	return &CreateUnifiedIDResponse{
		UnifiedId:  identityToProto(identity),
		TrustScore: trustScoreToProto(trustScore),
		Wallet:     walletToProto(wallet),
	}, nil
}

// GetUnifiedID retrieves a UnifiedID by DID
func (s *IdentityServer) GetUnifiedID(ctx context.Context, req *GetUnifiedIDRequest) (*GetUnifiedIDResponse, error) {
	if req.Did == "" {
		return nil, status.Error(codes.InvalidArgument, "did is required")
	}

	identity, err := s.svc.GetIdentity(ctx, req.Did)
	if err != nil {
		return nil, status.Errorf(codes.NotFound, "identity not found: %v", err)
	}

	return &GetUnifiedIDResponse{
		UnifiedId: identityToProto(identity),
	}, nil
}

// ResolveLineage resolves the identity lineage chain
func (s *IdentityServer) ResolveLineage(ctx context.Context, req *ResolveLineageRequest) (*ResolveLineageResponse, error) {
	if req.Did == "" {
		return nil, status.Error(codes.InvalidArgument, "did is required")
	}

	lineage, err := s.svc.GetAgentLineage(ctx, req.Did)
	if err != nil {
		return nil, status.Errorf(codes.NotFound, "lineage not found: %v", err)
	}

	// Build lineage response
	var lineageIDs []*UnifiedID
	var trustInheritance []float64

	// Add current identity
	currentIdentity, err := s.svc.GetIdentity(ctx, req.Did)
	if err != nil {
		return nil, status.Errorf(codes.Internal, "failed to get identity: %v", err)
	}
	lineageIDs = append(lineageIDs, identityToProto(currentIdentity))
	trustInheritance = append(trustInheritance, 1.0) // Full trust for self

	// Add ancestors
	inheritanceFactor := domain.InheritedTrust
	for _, ancestorDID := range lineage.Ancestors {
		ancestor, err := s.svc.GetIdentity(ctx, ancestorDID)
		if err != nil {
			continue // Skip if ancestor not found
		}
		lineageIDs = append(lineageIDs, identityToProto(ancestor))
		trustInheritance = append(trustInheritance, inheritanceFactor)
		inheritanceFactor *= domain.InheritedTrust
	}

	return &ResolveLineageResponse{
		Lineage:          lineageIDs,
		TrustInheritance: trustInheritance,
	}, nil
}

// GetTrustScore retrieves the trust score for an entity
func (s *IdentityServer) GetTrustScore(ctx context.Context, req *GetTrustScoreRequest) (*GetTrustScoreResponse, error) {
	if req.Did == "" {
		return nil, status.Error(codes.InvalidArgument, "did is required")
	}

	trustScore, err := s.svc.GetTrustScore(ctx, req.Did)
	if err != nil {
		return nil, status.Errorf(codes.NotFound, "trust score not found: %v", err)
	}

	return &GetTrustScoreResponse{
		TrustScore: trustScoreToProto(trustScore),
	}, nil
}

// UpdateTrustScore updates the trust score for an entity
func (s *IdentityServer) UpdateTrustScore(ctx context.Context, req *UpdateTrustScoreRequest) (*UpdateTrustScoreResponse, error) {
	if req.Did == "" {
		return nil, status.Error(codes.InvalidArgument, "did is required")
	}

	// Handle verification update
	if req.VerificationUpdate != nil {
		err := s.svc.RecordVerificationOutcome(ctx, req.Did, req.VerificationUpdate.Success, 0)
		if err != nil {
			return nil, status.Errorf(codes.Internal, "failed to update trust score: %v", err)
		}
	}

	// Get updated trust score
	trustScore, err := s.svc.GetTrustScore(ctx, req.Did)
	if err != nil {
		return nil, status.Errorf(codes.Internal, "failed to get trust score: %v", err)
	}

	return &UpdateTrustScoreResponse{
		TrustScore: trustScoreToProto(trustScore),
	}, nil
}

// GetWallet retrieves the HC wallet for an entity
func (s *IdentityServer) GetWallet(ctx context.Context, req *GetWalletRequest) (*GetWalletResponse, error) {
	if req.Did == "" {
		return nil, status.Error(codes.InvalidArgument, "did is required")
	}

	wallet, err := s.svc.GetHCWallet(ctx, req.Did)
	if err != nil {
		return nil, status.Errorf(codes.NotFound, "wallet not found: %v", err)
	}

	return &GetWalletResponse{
		Wallet: walletToProto(wallet),
	}, nil
}

// CreditWallet credits HC to a wallet
func (s *IdentityServer) CreditWallet(ctx context.Context, req *CreditWalletRequest) (*CreditWalletResponse, error) {
	if req.Did == "" {
		return nil, status.Error(codes.InvalidArgument, "did is required")
	}
	if req.Amount == "" {
		return nil, status.Error(codes.InvalidArgument, "amount is required")
	}

	amount, err := decimal.NewFromString(req.Amount)
	if err != nil {
		return nil, status.Errorf(codes.InvalidArgument, "invalid amount: %v", err)
	}

	if amount.LessThanOrEqual(decimal.Zero) {
		return nil, status.Error(codes.InvalidArgument, "amount must be positive")
	}

	err = s.svc.CreditHC(ctx, req.Did, amount)
	if err != nil {
		return nil, status.Errorf(codes.Internal, "failed to credit wallet: %v", err)
	}

	wallet, err := s.svc.GetHCWallet(ctx, req.Did)
	if err != nil {
		return nil, status.Errorf(codes.Internal, "failed to get wallet: %v", err)
	}

	return &CreditWalletResponse{
		Wallet: walletToProto(wallet),
	}, nil
}

// DebitWallet debits HC from a wallet
func (s *IdentityServer) DebitWallet(ctx context.Context, req *DebitWalletRequest) (*DebitWalletResponse, error) {
	if req.Did == "" {
		return nil, status.Error(codes.InvalidArgument, "did is required")
	}
	if req.Amount == "" {
		return nil, status.Error(codes.InvalidArgument, "amount is required")
	}

	amount, err := decimal.NewFromString(req.Amount)
	if err != nil {
		return nil, status.Errorf(codes.InvalidArgument, "invalid amount: %v", err)
	}

	if amount.LessThanOrEqual(decimal.Zero) {
		return nil, status.Error(codes.InvalidArgument, "amount must be positive")
	}

	// Lock and release to simulate debit
	err = s.svc.LockHCForEscrow(ctx, req.Did, amount)
	if err != nil {
		return nil, status.Errorf(codes.FailedPrecondition, "failed to debit wallet: %v", err)
	}

	err = s.svc.ReleaseHCFromEscrow(ctx, req.Did, amount)
	if err != nil {
		return nil, status.Errorf(codes.Internal, "failed to release from escrow: %v", err)
	}

	wallet, err := s.svc.GetHCWallet(ctx, req.Did)
	if err != nil {
		return nil, status.Errorf(codes.Internal, "failed to get wallet: %v", err)
	}

	return &DebitWalletResponse{
		Wallet: walletToProto(wallet),
	}, nil
}

// LockWallet locks HC in escrow
func (s *IdentityServer) LockWallet(ctx context.Context, req *LockWalletRequest) (*LockWalletResponse, error) {
	if req.Did == "" {
		return nil, status.Error(codes.InvalidArgument, "did is required")
	}
	if req.Amount == "" {
		return nil, status.Error(codes.InvalidArgument, "amount is required")
	}

	amount, err := decimal.NewFromString(req.Amount)
	if err != nil {
		return nil, status.Errorf(codes.InvalidArgument, "invalid amount: %v", err)
	}

	err = s.svc.LockHCForEscrow(ctx, req.Did, amount)
	if err != nil {
		return nil, status.Errorf(codes.FailedPrecondition, "failed to lock wallet: %v", err)
	}

	wallet, err := s.svc.GetHCWallet(ctx, req.Did)
	if err != nil {
		return nil, status.Errorf(codes.Internal, "failed to get wallet: %v", err)
	}

	return &LockWalletResponse{
		Wallet: walletToProto(wallet),
	}, nil
}

// ReleaseWallet releases HC from escrow
func (s *IdentityServer) ReleaseWallet(ctx context.Context, req *ReleaseWalletRequest) (*ReleaseWalletResponse, error) {
	if req.Did == "" {
		return nil, status.Error(codes.InvalidArgument, "did is required")
	}
	if req.Amount == "" {
		return nil, status.Error(codes.InvalidArgument, "amount is required")
	}

	amount, err := decimal.NewFromString(req.Amount)
	if err != nil {
		return nil, status.Errorf(codes.InvalidArgument, "invalid amount: %v", err)
	}

	// If target DID specified, transfer to them
	if req.TargetDid != nil && *req.TargetDid != "" {
		// Release from source
		err = s.svc.ReleaseHCFromEscrow(ctx, req.Did, amount)
		if err != nil {
			return nil, status.Errorf(codes.Internal, "failed to release escrow: %v", err)
		}
		// Credit to target
		err = s.svc.CreditHC(ctx, *req.TargetDid, amount)
		if err != nil {
			return nil, status.Errorf(codes.Internal, "failed to credit target: %v", err)
		}
	} else {
		// Refund back to available
		err = s.svc.RefundHCFromEscrow(ctx, req.Did, amount)
		if err != nil {
			return nil, status.Errorf(codes.Internal, "failed to refund escrow: %v", err)
		}
	}

	wallet, err := s.svc.GetHCWallet(ctx, req.Did)
	if err != nil {
		return nil, status.Errorf(codes.Internal, "failed to get wallet: %v", err)
	}

	return &ReleaseWalletResponse{
		Wallet: walletToProto(wallet),
	}, nil
}

// VerifySignature verifies a signature against an entity's public key
func (s *IdentityServer) VerifySignature(ctx context.Context, req *VerifySignatureRequest) (*VerifySignatureResponse, error) {
	if req.Did == "" {
		return nil, status.Error(codes.InvalidArgument, "did is required")
	}
	if len(req.Message) == 0 {
		return nil, status.Error(codes.InvalidArgument, "message is required")
	}
	if len(req.Signature) == 0 {
		return nil, status.Error(codes.InvalidArgument, "signature is required")
	}

	valid, err := s.svc.VerifyDIDSignature(ctx, req.Did, req.Message, req.Signature)
	if err != nil {
		return nil, status.Errorf(codes.Internal, "failed to verify signature: %v", err)
	}

	return &VerifySignatureResponse{
		Valid: valid,
	}, nil
}

// Helper functions for proto conversion

func protoToEntityType(et EntityType) domain.EntityType {
	switch et {
	case EntityType_ENTITY_TYPE_HUMAN:
		return domain.EntityTypeHuman
	case EntityType_ENTITY_TYPE_AGENT:
		return domain.EntityTypeAgent
	case EntityType_ENTITY_TYPE_ORGANIZATION:
		return domain.EntityTypeOrganization
	default:
		return domain.EntityTypeUnspecified
	}
}

func entityTypeToProto(et domain.EntityType) EntityType {
	switch et {
	case domain.EntityTypeHuman:
		return EntityType_ENTITY_TYPE_HUMAN
	case domain.EntityTypeAgent:
		return EntityType_ENTITY_TYPE_AGENT
	case domain.EntityTypeOrganization:
		return EntityType_ENTITY_TYPE_ORGANIZATION
	default:
		return EntityType_ENTITY_TYPE_UNSPECIFIED
	}
}

func identityToProto(id *domain.UnifiedID) *UnifiedID {
	if id == nil {
		return nil
	}
	return &UnifiedID{
		Did:        id.DID,
		EntityType: entityTypeToProto(id.EntityType),
		ParentDid:  id.ParentDID,
		CreatedAt:  id.CreatedAt.UnixMilli(),
		PublicKey:  id.PublicKey,
	}
}

func trustScoreToProto(ts *domain.TrustScore) *TrustScore {
	if ts == nil {
		return nil
	}
	return &TrustScore{
		Score: ts.Score,
		Components: &TrustComponents{
			VerificationScore: ts.Components.VerificationScore,
			DisputePenalty:    ts.Components.DisputePenalty,
			SlaScore:          ts.Components.SLAScore,
			NetworkScore:      ts.Components.NetworkScore,
		},
		UpdatedAt:        ts.UpdatedAt.UnixMilli(),
		VerifiedOutcomes: ts.VerifiedOutcomes,
		DisputeRate:      ts.DisputeRate,
		Version:          ts.Version,
	}
}

func walletToProto(w *domain.HCWallet) *HcWallet {
	if w == nil {
		return nil
	}
	return &HcWallet{
		OwnerDid:  w.OwnerDID,
		Available: w.Available.String(),
		Locked:    w.Locked.String(),
		ExpiresAt: w.ExpiresAt.UnixMilli(),
		Version:   w.Version,
		UpdatedAt: w.UpdatedAt.UnixMilli(),
	}
}

// generateDIDKey generates a did:key from an Ed25519 public key
func generateDIDKey(publicKey ed25519.PublicKey) string {
	// Multicodec prefix for Ed25519 public key (0xed01)
	multicodec := []byte{0xed, 0x01}
	encoded := append(multicodec, publicKey...)

	// Base58btc encode (simplified - using base64url for now)
	return "did:key:z" + base64.RawURLEncoding.EncodeToString(encoded)
}
