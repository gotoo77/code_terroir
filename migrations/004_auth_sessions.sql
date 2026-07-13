CREATE TABLE auth_sessions (
    id UUID PRIMARY KEY,
    family_id UUID NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    replaced_by UUID REFERENCES auth_sessions(id) ON DELETE SET NULL,
    last_used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_auth_sessions_user_active
    ON auth_sessions (user_id, expires_at)
    WHERE revoked_at IS NULL;

CREATE INDEX idx_auth_sessions_family
    ON auth_sessions (family_id);
