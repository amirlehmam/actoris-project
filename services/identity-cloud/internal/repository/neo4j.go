// Package repository provides Neo4j database access for identity management
package repository

import (
	"context"
	"fmt"
	"time"

	"github.com/actoris/actoris/services/identity-cloud/internal/domain"
	"github.com/neo4j/neo4j-go-driver/v5/neo4j"
	"github.com/shopspring/decimal"
)

// Neo4jRepository provides Neo4j database operations for identity management
type Neo4jRepository struct {
	driver neo4j.DriverWithContext
}

// NewNeo4jRepository creates a new Neo4j repository
func NewNeo4jRepository(ctx context.Context, uri, username, password string) (*Neo4jRepository, error) {
	driver, err := neo4j.NewDriverWithContext(uri, neo4j.BasicAuth(username, password, ""))
	if err != nil {
		return nil, fmt.Errorf("failed to create neo4j driver: %w", err)
	}

	// Verify connectivity with timeout
	verifyCtx, cancel := context.WithTimeout(ctx, 10*time.Second)
	defer cancel()

	if err := driver.VerifyConnectivity(verifyCtx); err != nil {
		return nil, fmt.Errorf("failed to verify neo4j connectivity: %w", err)
	}

	return &Neo4jRepository{driver: driver}, nil
}

// ExecuteWrite executes a write query with parameters
func (r *Neo4jRepository) ExecuteWrite(ctx context.Context, query string, params map[string]any) error {
	session := r.driver.NewSession(ctx, neo4j.SessionConfig{AccessMode: neo4j.AccessModeWrite})
	defer session.Close(ctx)

	_, err := session.ExecuteWrite(ctx, func(tx neo4j.ManagedTransaction) (any, error) {
		result, err := tx.Run(ctx, query, params)
		if err != nil {
			return nil, err
		}
		return result.Consume(ctx)
	})
	return err
}

// Close closes the Neo4j driver
func (r *Neo4jRepository) Close(ctx context.Context) error {
	return r.driver.Close(ctx)
}

// CreateIdentity creates a new UnifiedID in the graph
func (r *Neo4jRepository) CreateIdentity(ctx context.Context, id *domain.UnifiedID) error {
	session := r.driver.NewSession(ctx, neo4j.SessionConfig{AccessMode: neo4j.AccessModeWrite})
	defer session.Close(ctx)

	_, err := session.ExecuteWrite(ctx, func(tx neo4j.ManagedTransaction) (any, error) {
		query := `
			CREATE (i:Identity {
				did: $did,
				entity_type: $entity_type,
				created_at: datetime($created_at),
				public_key: $public_key
			})
			WITH i
			// If parent exists, create SPAWNED relationship
			OPTIONAL MATCH (parent:Identity {did: $parent_did})
			FOREACH (p IN CASE WHEN parent IS NOT NULL THEN [parent] ELSE [] END |
				CREATE (p)-[:SPAWNED {created_at: datetime($created_at)}]->(i)
			)
			// Create initial trust score
			CREATE (i)-[:HAS_TRUST]->(t:TrustScore {
				score: 500,
				verification_score: 200,
				dispute_penalty: 0,
				sla_score: 100,
				network_score: 200,
				updated_at: datetime($created_at),
				verified_outcomes: 0,
				dispute_rate: 0.0,
				version: 1
			})
			// Create HC wallet
			CREATE (i)-[:OWNS_WALLET]->(w:HCWallet {
				available: '0',
				locked: '0',
				expires_at: datetime() + duration('P30D'),
				version: 1,
				updated_at: datetime($created_at)
			})
			RETURN i.did
		`
		params := map[string]any{
			"did":         id.DID,
			"entity_type": id.EntityType.String(),
			"created_at":  id.CreatedAt.Format(time.RFC3339),
			"public_key":  id.PublicKey,
			"parent_did":  nil,
		}
		if id.ParentDID != nil {
			params["parent_did"] = *id.ParentDID
		}

		result, err := tx.Run(ctx, query, params)
		if err != nil {
			return nil, err
		}
		return result.Consume(ctx)
	})

	return err
}

// GetIdentity retrieves a UnifiedID by DID
func (r *Neo4jRepository) GetIdentity(ctx context.Context, did string) (*domain.UnifiedID, error) {
	session := r.driver.NewSession(ctx, neo4j.SessionConfig{AccessMode: neo4j.AccessModeRead})
	defer session.Close(ctx)

	result, err := session.ExecuteRead(ctx, func(tx neo4j.ManagedTransaction) (any, error) {
		query := `
			MATCH (i:Identity {did: $did})
			OPTIONAL MATCH (parent:Identity)-[:SPAWNED]->(i)
			RETURN i.did as did,
				   i.entity_type as entity_type,
				   i.created_at as created_at,
				   i.public_key as public_key,
				   parent.did as parent_did
		`
		result, err := tx.Run(ctx, query, map[string]any{"did": did})
		if err != nil {
			return nil, err
		}

		if result.Next(ctx) {
			record := result.Record()
			id := &domain.UnifiedID{
				DID:       record.Values[0].(string),
				PublicKey: record.Values[3].([]byte),
			}

			// Parse entity type
			switch record.Values[1].(string) {
			case "human":
				id.EntityType = domain.EntityTypeHuman
			case "agent":
				id.EntityType = domain.EntityTypeAgent
			case "organization":
				id.EntityType = domain.EntityTypeOrganization
			}

			// Parse created_at
			if createdAt, ok := record.Values[2].(neo4j.LocalDateTime); ok {
				id.CreatedAt = createdAt.Time()
			}

			// Parse parent_did
			if parentDID, ok := record.Values[4].(string); ok {
				id.ParentDID = &parentDID
			}

			return id, nil
		}

		return nil, nil
	})

	if err != nil {
		return nil, err
	}
	if result == nil {
		return nil, fmt.Errorf("identity not found: %s", did)
	}
	return result.(*domain.UnifiedID), nil
}

// GetTrustScore retrieves the trust score for an identity
func (r *Neo4jRepository) GetTrustScore(ctx context.Context, did string) (*domain.TrustScore, error) {
	session := r.driver.NewSession(ctx, neo4j.SessionConfig{AccessMode: neo4j.AccessModeRead})
	defer session.Close(ctx)

	result, err := session.ExecuteRead(ctx, func(tx neo4j.ManagedTransaction) (any, error) {
		query := `
			MATCH (i:Identity {did: $did})-[:HAS_TRUST]->(t:TrustScore)
			RETURN t.score as score,
				   t.verification_score as verification_score,
				   t.dispute_penalty as dispute_penalty,
				   t.sla_score as sla_score,
				   t.network_score as network_score,
				   t.updated_at as updated_at,
				   t.verified_outcomes as verified_outcomes,
				   t.dispute_rate as dispute_rate,
				   t.version as version
		`
		result, err := tx.Run(ctx, query, map[string]any{"did": did})
		if err != nil {
			return nil, err
		}

		if result.Next(ctx) {
			record := result.Record()
			trust := &domain.TrustScore{
				Score: uint32(record.Values[0].(int64)),
				Components: domain.TrustComponents{
					VerificationScore: uint32(record.Values[1].(int64)),
					DisputePenalty:    uint32(record.Values[2].(int64)),
					SLAScore:          uint32(record.Values[3].(int64)),
					NetworkScore:      uint32(record.Values[4].(int64)),
				},
				VerifiedOutcomes: uint64(record.Values[6].(int64)),
				DisputeRate:      record.Values[7].(float64),
				Version:          uint64(record.Values[8].(int64)),
			}

			if updatedAt, ok := record.Values[5].(neo4j.LocalDateTime); ok {
				trust.UpdatedAt = updatedAt.Time()
			}

			return trust, nil
		}

		return nil, nil
	})

	if err != nil {
		return nil, err
	}
	if result == nil {
		return nil, fmt.Errorf("trust score not found for: %s", did)
	}
	return result.(*domain.TrustScore), nil
}

// UpdateTrustScore updates the trust score for an identity
func (r *Neo4jRepository) UpdateTrustScore(ctx context.Context, did string, update *domain.TrustScore) error {
	session := r.driver.NewSession(ctx, neo4j.SessionConfig{AccessMode: neo4j.AccessModeWrite})
	defer session.Close(ctx)

	_, err := session.ExecuteWrite(ctx, func(tx neo4j.ManagedTransaction) (any, error) {
		// Use optimistic locking with version check
		query := `
			MATCH (i:Identity {did: $did})-[:HAS_TRUST]->(t:TrustScore)
			WHERE t.version = $expected_version
			SET t.score = $score,
				t.verification_score = $verification_score,
				t.dispute_penalty = $dispute_penalty,
				t.sla_score = $sla_score,
				t.network_score = $network_score,
				t.updated_at = datetime(),
				t.verified_outcomes = $verified_outcomes,
				t.dispute_rate = $dispute_rate,
				t.version = t.version + 1
			RETURN t.version as new_version
		`
		params := map[string]any{
			"did":                did,
			"expected_version":   update.Version,
			"score":              update.Score,
			"verification_score": update.Components.VerificationScore,
			"dispute_penalty":    update.Components.DisputePenalty,
			"sla_score":          update.Components.SLAScore,
			"network_score":      update.Components.NetworkScore,
			"verified_outcomes":  update.VerifiedOutcomes,
			"dispute_rate":       update.DisputeRate,
		}

		result, err := tx.Run(ctx, query, params)
		if err != nil {
			return nil, err
		}

		if !result.Next(ctx) {
			return nil, fmt.Errorf("concurrent modification detected for trust score: %s", did)
		}

		return result.Consume(ctx)
	})

	return err
}

// RecordVerificationOutcome updates trust score based on verification result
func (r *Neo4jRepository) RecordVerificationOutcome(ctx context.Context, did string, passed bool, latencyMs uint32) error {
	session := r.driver.NewSession(ctx, neo4j.SessionConfig{AccessMode: neo4j.AccessModeWrite})
	defer session.Close(ctx)

	_, err := session.ExecuteWrite(ctx, func(tx neo4j.ManagedTransaction) (any, error) {
		query := `
			MATCH (i:Identity {did: $did})-[:HAS_TRUST]->(t:TrustScore)
			SET t.verified_outcomes = t.verified_outcomes + 1,
				t.updated_at = datetime(),
				t.version = t.version + 1,
				// Adjust verification score based on outcome
				t.verification_score = CASE
					WHEN $passed THEN CASE WHEN t.verification_score < 400 THEN t.verification_score + 1 ELSE 400 END
					ELSE CASE WHEN t.verification_score > 0 THEN t.verification_score - 2 ELSE 0 END
				END,
				// Adjust SLA score based on latency (target: 2000ms)
				t.sla_score = CASE
					WHEN $latency_ms <= 2000 THEN CASE WHEN t.sla_score < 200 THEN t.sla_score + 1 ELSE 200 END
					ELSE CASE WHEN t.sla_score > 0 THEN t.sla_score - 1 ELSE 0 END
				END,
				// Recalculate total score
				t.score = t.verification_score + t.sla_score + t.network_score - t.dispute_penalty
			RETURN t.score
		`
		result, err := tx.Run(ctx, query, map[string]any{
			"did":        did,
			"passed":     passed,
			"latency_ms": latencyMs,
		})
		if err != nil {
			return nil, err
		}
		return result.Consume(ctx)
	})

	return err
}

// GetHCWallet retrieves the HC wallet for an identity
func (r *Neo4jRepository) GetHCWallet(ctx context.Context, did string) (*domain.HCWallet, error) {
	session := r.driver.NewSession(ctx, neo4j.SessionConfig{AccessMode: neo4j.AccessModeRead})
	defer session.Close(ctx)

	result, err := session.ExecuteRead(ctx, func(tx neo4j.ManagedTransaction) (any, error) {
		query := `
			MATCH (i:Identity {did: $did})-[:OWNS_WALLET]->(w:HCWallet)
			RETURN w.available as available,
				   w.locked as locked,
				   w.expires_at as expires_at,
				   w.version as version,
				   w.updated_at as updated_at
		`
		result, err := tx.Run(ctx, query, map[string]any{"did": did})
		if err != nil {
			return nil, err
		}

		if result.Next(ctx) {
			record := result.Record()

			available, _ := decimal.NewFromString(record.Values[0].(string))
			locked, _ := decimal.NewFromString(record.Values[1].(string))

			wallet := &domain.HCWallet{
				OwnerDID:  did,
				Available: available,
				Locked:    locked,
				Version:   uint64(record.Values[3].(int64)),
			}

			if expiresAt, ok := record.Values[2].(neo4j.LocalDateTime); ok {
				wallet.ExpiresAt = expiresAt.Time()
			}
			if updatedAt, ok := record.Values[4].(neo4j.LocalDateTime); ok {
				wallet.UpdatedAt = updatedAt.Time()
			}

			return wallet, nil
		}

		return nil, nil
	})

	if err != nil {
		return nil, err
	}
	if result == nil {
		return nil, fmt.Errorf("wallet not found for: %s", did)
	}
	return result.(*domain.HCWallet), nil
}

// LockHC locks HC for an escrow transaction
func (r *Neo4jRepository) LockHC(ctx context.Context, did string, amount decimal.Decimal, version uint64) error {
	session := r.driver.NewSession(ctx, neo4j.SessionConfig{AccessMode: neo4j.AccessModeWrite})
	defer session.Close(ctx)

	_, err := session.ExecuteWrite(ctx, func(tx neo4j.ManagedTransaction) (any, error) {
		query := `
			MATCH (i:Identity {did: $did})-[:OWNS_WALLET]->(w:HCWallet)
			WHERE w.version = $expected_version
			  AND toFloat(w.available) >= $amount
			  AND w.expires_at > datetime()
			SET w.available = toString(toFloat(w.available) - $amount),
				w.locked = toString(toFloat(w.locked) + $amount),
				w.updated_at = datetime(),
				w.version = w.version + 1
			RETURN w.version as new_version
		`
		amountFloat, _ := amount.Float64()
		result, err := tx.Run(ctx, query, map[string]any{
			"did":              did,
			"expected_version": version,
			"amount":           amountFloat,
		})
		if err != nil {
			return nil, err
		}

		if !result.Next(ctx) {
			return nil, fmt.Errorf("insufficient balance or concurrent modification")
		}

		return result.Consume(ctx)
	})

	return err
}

// ReleaseHC releases locked HC after successful transaction
func (r *Neo4jRepository) ReleaseHC(ctx context.Context, did string, amount decimal.Decimal) error {
	session := r.driver.NewSession(ctx, neo4j.SessionConfig{AccessMode: neo4j.AccessModeWrite})
	defer session.Close(ctx)

	_, err := session.ExecuteWrite(ctx, func(tx neo4j.ManagedTransaction) (any, error) {
		query := `
			MATCH (i:Identity {did: $did})-[:OWNS_WALLET]->(w:HCWallet)
			WHERE toFloat(w.locked) >= $amount
			SET w.locked = toString(toFloat(w.locked) - $amount),
				w.updated_at = datetime(),
				w.version = w.version + 1
			RETURN w.version
		`
		amountFloat, _ := amount.Float64()
		result, err := tx.Run(ctx, query, map[string]any{
			"did":    did,
			"amount": amountFloat,
		})
		if err != nil {
			return nil, err
		}
		return result.Consume(ctx)
	})

	return err
}

// RefundHC refunds locked HC back to available balance
func (r *Neo4jRepository) RefundHC(ctx context.Context, did string, amount decimal.Decimal) error {
	session := r.driver.NewSession(ctx, neo4j.SessionConfig{AccessMode: neo4j.AccessModeWrite})
	defer session.Close(ctx)

	_, err := session.ExecuteWrite(ctx, func(tx neo4j.ManagedTransaction) (any, error) {
		query := `
			MATCH (i:Identity {did: $did})-[:OWNS_WALLET]->(w:HCWallet)
			WHERE toFloat(w.locked) >= $amount
			SET w.available = toString(toFloat(w.available) + $amount),
				w.locked = toString(toFloat(w.locked) - $amount),
				w.updated_at = datetime(),
				w.version = w.version + 1
			RETURN w.version
		`
		amountFloat, _ := amount.Float64()
		result, err := tx.Run(ctx, query, map[string]any{
			"did":    did,
			"amount": amountFloat,
		})
		if err != nil {
			return nil, err
		}
		return result.Consume(ctx)
	})

	return err
}

// CreditHC adds HC to a wallet
func (r *Neo4jRepository) CreditHC(ctx context.Context, did string, amount decimal.Decimal) error {
	session := r.driver.NewSession(ctx, neo4j.SessionConfig{AccessMode: neo4j.AccessModeWrite})
	defer session.Close(ctx)

	_, err := session.ExecuteWrite(ctx, func(tx neo4j.ManagedTransaction) (any, error) {
		query := `
			MATCH (i:Identity {did: $did})-[:OWNS_WALLET]->(w:HCWallet)
			SET w.available = toString(toFloat(w.available) + $amount),
				w.expires_at = CASE
					WHEN w.expires_at < datetime() + duration('P30D')
					THEN datetime() + duration('P30D')
					ELSE w.expires_at
				END,
				w.updated_at = datetime(),
				w.version = w.version + 1
			RETURN w.available
		`
		amountFloat, _ := amount.Float64()
		result, err := tx.Run(ctx, query, map[string]any{
			"did":    did,
			"amount": amountFloat,
		})
		if err != nil {
			return nil, err
		}
		return result.Consume(ctx)
	})

	return err
}

// GetAgentLineage retrieves the full lineage of an agent
func (r *Neo4jRepository) GetAgentLineage(ctx context.Context, did string) (*domain.AgentLineage, error) {
	session := r.driver.NewSession(ctx, neo4j.SessionConfig{AccessMode: neo4j.AccessModeRead})
	defer session.Close(ctx)

	result, err := session.ExecuteRead(ctx, func(tx neo4j.ManagedTransaction) (any, error) {
		query := `
			MATCH path = (root:Identity)-[:SPAWNED*0..]->(agent:Identity {did: $did})
			WHERE NOT ()-[:SPAWNED]->(root)
			WITH agent, root, nodes(path) as ancestors, length(path) as depth
			OPTIONAL MATCH (parent:Identity)-[:SPAWNED]->(agent)
			RETURN agent.did as did,
				   parent.did as parent_did,
				   root.did as root_did,
				   depth,
				   agent.created_at as spawned_at,
				   [n in ancestors | n.did] as ancestor_dids
		`
		result, err := tx.Run(ctx, query, map[string]any{"did": did})
		if err != nil {
			return nil, err
		}

		if result.Next(ctx) {
			record := result.Record()

			lineage := &domain.AgentLineage{
				AgentDID: record.Values[0].(string),
				RootDID:  record.Values[2].(string),
				Depth:    int(record.Values[3].(int64)),
			}

			if parentDID, ok := record.Values[1].(string); ok {
				lineage.ParentDID = &parentDID
			}

			if spawnedAt, ok := record.Values[4].(neo4j.LocalDateTime); ok {
				lineage.SpawnedAt = spawnedAt.Time()
			}

			if ancestors, ok := record.Values[5].([]any); ok {
				for _, a := range ancestors {
					if s, ok := a.(string); ok {
						lineage.Ancestors = append(lineage.Ancestors, s)
					}
				}
			}

			return lineage, nil
		}

		return nil, nil
	})

	if err != nil {
		return nil, err
	}
	if result == nil {
		return nil, fmt.Errorf("lineage not found for: %s", did)
	}
	return result.(*domain.AgentLineage), nil
}

// GetSpawnedAgents retrieves all agents spawned by an identity
func (r *Neo4jRepository) GetSpawnedAgents(ctx context.Context, did string) ([]domain.UnifiedID, error) {
	session := r.driver.NewSession(ctx, neo4j.SessionConfig{AccessMode: neo4j.AccessModeRead})
	defer session.Close(ctx)

	result, err := session.ExecuteRead(ctx, func(tx neo4j.ManagedTransaction) (any, error) {
		query := `
			MATCH (parent:Identity {did: $did})-[:SPAWNED]->(child:Identity)
			RETURN child.did as did,
				   child.entity_type as entity_type,
				   child.created_at as created_at,
				   child.public_key as public_key
		`
		result, err := tx.Run(ctx, query, map[string]any{"did": did})
		if err != nil {
			return nil, err
		}

		var agents []domain.UnifiedID
		for result.Next(ctx) {
			record := result.Record()
			agent := domain.UnifiedID{
				DID:       record.Values[0].(string),
				ParentDID: &did,
			}

			switch record.Values[1].(string) {
			case "human":
				agent.EntityType = domain.EntityTypeHuman
			case "agent":
				agent.EntityType = domain.EntityTypeAgent
			case "organization":
				agent.EntityType = domain.EntityTypeOrganization
			}

			if createdAt, ok := record.Values[2].(neo4j.LocalDateTime); ok {
				agent.CreatedAt = createdAt.Time()
			}
			if pk, ok := record.Values[3].([]byte); ok {
				agent.PublicKey = pk
			}

			agents = append(agents, agent)
		}

		return agents, nil
	})

	if err != nil {
		return nil, err
	}
	return result.([]domain.UnifiedID), nil
}

// CreateConstraintsAndIndexes creates necessary Neo4j constraints and indexes
func (r *Neo4jRepository) CreateConstraintsAndIndexes(ctx context.Context) error {
	session := r.driver.NewSession(ctx, neo4j.SessionConfig{AccessMode: neo4j.AccessModeWrite})
	defer session.Close(ctx)

	constraints := []string{
		"CREATE CONSTRAINT identity_did IF NOT EXISTS FOR (i:Identity) REQUIRE i.did IS UNIQUE",
	}

	indexes := []string{
		"CREATE INDEX identity_entity_type IF NOT EXISTS FOR (i:Identity) ON (i.entity_type)",
		"CREATE INDEX identity_created_at IF NOT EXISTS FOR (i:Identity) ON (i.created_at)",
	}

	for _, constraint := range constraints {
		_, err := session.Run(ctx, constraint, nil)
		if err != nil {
			return fmt.Errorf("failed to create constraint: %w", err)
		}
	}

	for _, index := range indexes {
		_, err := session.Run(ctx, index, nil)
		if err != nil {
			return fmt.Errorf("failed to create index: %w", err)
		}
	}

	return nil
}
