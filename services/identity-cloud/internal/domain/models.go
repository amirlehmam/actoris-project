// Package domain contains the core domain models for IdentityCloud
package domain

import (
	"time"

	"github.com/google/uuid"
	"github.com/shopspring/decimal"
)

// EntityType represents the type of entity
type EntityType int

const (
	EntityTypeUnspecified EntityType = iota
	EntityTypeHuman
	EntityTypeAgent
	EntityTypeOrganization
)

func (e EntityType) String() string {
	switch e {
	case EntityTypeHuman:
		return "human"
	case EntityTypeAgent:
		return "agent"
	case EntityTypeOrganization:
		return "organization"
	default:
		return "unspecified"
	}
}

// UnifiedID represents a W3C DID-based identity
type UnifiedID struct {
	DID        string     `json:"did"`
	EntityType EntityType `json:"entity_type"`
	ParentDID  *string    `json:"parent_did,omitempty"`
	CreatedAt  time.Time  `json:"created_at"`
	PublicKey  []byte     `json:"public_key"`
}

// TrustComponents holds the breakdown of trust score components
type TrustComponents struct {
	VerificationScore uint32 `json:"verification_score"` // 0-400
	DisputePenalty    uint32 `json:"dispute_penalty"`    // 0-200
	SLAScore          uint32 `json:"sla_score"`          // 0-200
	NetworkScore      uint32 `json:"network_score"`      // 0-200
}

// TrustScore represents an entity's trust score with full breakdown
type TrustScore struct {
	Score            uint32          `json:"score"` // 0-1000
	Components       TrustComponents `json:"components"`
	UpdatedAt        time.Time       `json:"updated_at"`
	VerifiedOutcomes uint64          `json:"verified_outcomes"`
	DisputeRate      float64         `json:"dispute_rate"` // 0.0 - 1.0
	Version          uint64          `json:"version"`      // For optimistic concurrency
}

// Constants for trust score calculation
const (
	MaxScore         = 1000
	MaxDiscountRate  = 0.20 // 20% maximum discount for high trust
	GraceEpochs      = 10
	InheritedTrust   = 0.30 // Spawned agents inherit 30% of parent trust
	DecayFactor      = 0.005
	MinInheritedTau  = 0.10
)

// Tau returns the normalized trust score (0.0 to 1.0)
func (t *TrustScore) Tau() float64 {
	return float64(t.Score) / float64(MaxScore)
}

// DiscountRate returns the discount rate based on tau (up to 20%)
func (t *TrustScore) DiscountRate() float64 {
	discount := t.Tau() * MaxDiscountRate
	if discount > MaxDiscountRate {
		return MaxDiscountRate
	}
	return discount
}

// IsHighTrust returns true if trust score indicates high reliability
func (t *TrustScore) IsHighTrust() bool {
	return t.Score >= 800
}

// IsMediumTrust returns true if trust score indicates medium reliability
func (t *TrustScore) IsMediumTrust() bool {
	return t.Score >= 500 && t.Score < 800
}

// IsLowTrust returns true if trust score indicates low reliability
func (t *TrustScore) IsLowTrust() bool {
	return t.Score < 500
}

// InheritedTauForChild calculates the trust score a child entity inherits
func (t *TrustScore) InheritedTauForChild() float64 {
	inherited := t.Tau() * InheritedTrust
	if inherited < MinInheritedTau {
		return MinInheritedTau
	}
	return inherited
}

// HCWallet represents a Harness Credits wallet
type HCWallet struct {
	ID          uuid.UUID       `json:"id"`
	OwnerDID    string          `json:"owner_did"`
	Available   decimal.Decimal `json:"available"`  // Available HC balance
	Locked      decimal.Decimal `json:"locked"`     // Locked in escrow
	ExpiresAt   time.Time       `json:"expires_at"` // 30-day expiry
	Version     uint64          `json:"version"`    // For optimistic concurrency
	UpdatedAt   time.Time       `json:"updated_at"`
}

// Total returns the total balance (available + locked)
func (w *HCWallet) Total() decimal.Decimal {
	return w.Available.Add(w.Locked)
}

// IsExpired checks if the wallet credits have expired
func (w *HCWallet) IsExpired() bool {
	return time.Now().After(w.ExpiresAt)
}

// CanLock checks if the wallet can lock the specified amount
func (w *HCWallet) CanLock(amount decimal.Decimal) bool {
	return w.Available.GreaterThanOrEqual(amount) && !w.IsExpired()
}

// IdentityGraph represents relationships between identities
type IdentityGraph struct {
	Nodes []UnifiedID      `json:"nodes"`
	Edges []IdentityEdge   `json:"edges"`
}

// IdentityEdge represents a relationship between two identities
type IdentityEdge struct {
	FromDID      string    `json:"from_did"`
	ToDID        string    `json:"to_did"`
	Relationship string    `json:"relationship"` // SPAWNED, DELEGATED, TRUSTED
	CreatedAt    time.Time `json:"created_at"`
}

// RelationshipType constants
const (
	RelationshipSpawned   = "SPAWNED"   // Parent spawned child agent
	RelationshipDelegated = "DELEGATED" // Authority delegation
	RelationshipTrusted   = "TRUSTED"   // Trust relationship
)

// AgentLineage tracks the spawn history of an agent
type AgentLineage struct {
	AgentDID   string     `json:"agent_did"`
	ParentDID  *string    `json:"parent_did,omitempty"`
	RootDID    string     `json:"root_did"` // Original human/org
	Depth      int        `json:"depth"`    // Spawn depth from root
	SpawnedAt  time.Time  `json:"spawned_at"`
	Ancestors  []string   `json:"ancestors"` // Full lineage chain
}
