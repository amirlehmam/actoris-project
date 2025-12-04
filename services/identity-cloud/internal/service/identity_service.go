// Package service provides business logic for identity management
package service

import (
	"context"
	"crypto/ed25519"
	"encoding/base64"
	"fmt"
	"time"

	"github.com/actoris/actoris/services/identity-cloud/internal/domain"
	"github.com/actoris/actoris/services/identity-cloud/internal/repository"
	"github.com/shopspring/decimal"
)

// IdentityService provides business logic for identity management
type IdentityService struct {
	repo *repository.Neo4jRepository
}

// NewIdentityService creates a new identity service
func NewIdentityService(repo *repository.Neo4jRepository) *IdentityService {
	return &IdentityService{repo: repo}
}

// CreateIdentity creates a new identity with DID generation
func (s *IdentityService) CreateIdentity(ctx context.Context, entityType domain.EntityType, parentDID *string) (*domain.UnifiedID, error) {
	// Generate Ed25519 keypair
	publicKey, _, err := ed25519.GenerateKey(nil)
	if err != nil {
		return nil, fmt.Errorf("failed to generate keypair: %w", err)
	}

	// Generate did:key from public key
	did := generateDIDKey(publicKey)

	identity := &domain.UnifiedID{
		DID:        did,
		EntityType: entityType,
		ParentDID:  parentDID,
		CreatedAt:  time.Now().UTC(),
		PublicKey:  publicKey,
	}

	// Create in database
	if err := s.repo.CreateIdentity(ctx, identity); err != nil {
		return nil, fmt.Errorf("failed to create identity: %w", err)
	}

	// If spawned from parent, inherit trust score
	if parentDID != nil {
		if err := s.inheritParentTrust(ctx, did, *parentDID); err != nil {
			// Log but don't fail - identity was created
			fmt.Printf("Warning: failed to inherit parent trust: %v\n", err)
		}
	}

	return identity, nil
}

// GetIdentity retrieves an identity by DID
func (s *IdentityService) GetIdentity(ctx context.Context, did string) (*domain.UnifiedID, error) {
	return s.repo.GetIdentity(ctx, did)
}

// GetTrustScore retrieves the trust score for an identity
func (s *IdentityService) GetTrustScore(ctx context.Context, did string) (*domain.TrustScore, error) {
	return s.repo.GetTrustScore(ctx, did)
}

// RecordVerificationOutcome updates trust based on verification result
func (s *IdentityService) RecordVerificationOutcome(ctx context.Context, did string, passed bool, latencyMs uint32) error {
	return s.repo.RecordVerificationOutcome(ctx, did, passed, latencyMs)
}

// GetHCWallet retrieves the HC wallet for an identity
func (s *IdentityService) GetHCWallet(ctx context.Context, did string) (*domain.HCWallet, error) {
	return s.repo.GetHCWallet(ctx, did)
}

// LockHCForEscrow locks HC for an escrow transaction
func (s *IdentityService) LockHCForEscrow(ctx context.Context, did string, amount decimal.Decimal) error {
	wallet, err := s.repo.GetHCWallet(ctx, did)
	if err != nil {
		return err
	}

	if wallet.IsExpired() {
		return fmt.Errorf("wallet has expired credits")
	}

	if !wallet.CanLock(amount) {
		return fmt.Errorf("insufficient balance: available=%s, requested=%s", wallet.Available, amount)
	}

	return s.repo.LockHC(ctx, did, amount, wallet.Version)
}

// ReleaseHCFromEscrow releases HC after successful transaction
func (s *IdentityService) ReleaseHCFromEscrow(ctx context.Context, did string, amount decimal.Decimal) error {
	return s.repo.ReleaseHC(ctx, did, amount)
}

// RefundHCFromEscrow refunds HC back to available balance
func (s *IdentityService) RefundHCFromEscrow(ctx context.Context, did string, amount decimal.Decimal) error {
	return s.repo.RefundHC(ctx, did, amount)
}

// CreditHC adds HC to a wallet
func (s *IdentityService) CreditHC(ctx context.Context, did string, amount decimal.Decimal) error {
	return s.repo.CreditHC(ctx, did, amount)
}

// GetAgentLineage retrieves the full lineage of an agent
func (s *IdentityService) GetAgentLineage(ctx context.Context, did string) (*domain.AgentLineage, error) {
	return s.repo.GetAgentLineage(ctx, did)
}

// GetSpawnedAgents retrieves all agents spawned by an identity
func (s *IdentityService) GetSpawnedAgents(ctx context.Context, did string) ([]domain.UnifiedID, error) {
	return s.repo.GetSpawnedAgents(ctx, did)
}

// CalculateDiscount calculates the trust-based discount for pricing
func (s *IdentityService) CalculateDiscount(ctx context.Context, did string) (float64, error) {
	trust, err := s.repo.GetTrustScore(ctx, did)
	if err != nil {
		return 0, err
	}
	return trust.DiscountRate(), nil
}

// inheritParentTrust inherits trust score from parent identity
func (s *IdentityService) inheritParentTrust(ctx context.Context, childDID, parentDID string) error {
	parentTrust, err := s.repo.GetTrustScore(ctx, parentDID)
	if err != nil {
		return err
	}

	// Calculate inherited trust (30% of parent)
	inheritedTau := parentTrust.InheritedTauForChild()
	inheritedScore := uint32(inheritedTau * float64(domain.MaxScore))

	// Get child's current trust score
	childTrust, err := s.repo.GetTrustScore(ctx, childDID)
	if err != nil {
		return err
	}

	// Update with inherited values
	childTrust.Score = inheritedScore
	childTrust.Components.VerificationScore = uint32(float64(parentTrust.Components.VerificationScore) * domain.InheritedTrust)
	childTrust.Components.NetworkScore = uint32(float64(parentTrust.Components.NetworkScore) * domain.InheritedTrust)

	return s.repo.UpdateTrustScore(ctx, childDID, childTrust)
}

// generateDIDKey generates a did:key from an Ed25519 public key
func generateDIDKey(publicKey ed25519.PublicKey) string {
	// Multicodec prefix for Ed25519 public key (0xed01)
	multicodec := []byte{0xed, 0x01}
	encoded := append(multicodec, publicKey...)

	// Base58btc encode
	return "did:key:z" + base64.RawURLEncoding.EncodeToString(encoded)
}

// VerifyDIDSignature verifies a signature against a DID's public key
func (s *IdentityService) VerifyDIDSignature(ctx context.Context, did string, message, signature []byte) (bool, error) {
	identity, err := s.repo.GetIdentity(ctx, did)
	if err != nil {
		return false, err
	}

	if len(identity.PublicKey) != ed25519.PublicKeySize {
		return false, fmt.Errorf("invalid public key length")
	}

	return ed25519.Verify(identity.PublicKey, message, signature), nil
}
