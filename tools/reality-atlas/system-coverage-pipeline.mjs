import { compileSystemCoverage } from './system-coverage.mjs'
import { enrichSystemCoverage } from './system-coverage-relations.mjs'
import { enrichApiCoverage } from './system-coverage-api.mjs'
import { enrichEvidenceMaturity } from './system-coverage-evidence.mjs'
import { enrichLocalVerificationEvidence } from './local-verification-evidence.mjs'
import { enrichSystemCoverageDepth } from './system-coverage-depth.mjs'
import { enrichUiSemanticChains } from './system-coverage-ui-depth.mjs'
import { enrichDisplayLanguage } from './system-coverage-display-language.mjs'
import { enrichNamingTopology } from './system-coverage-naming.mjs'
import { unifySystemCoverage } from './system-coverage-unified.mjs'

export function compileFullSystemCoverage(root, options = {}) {
  return unifySystemCoverage(
    enrichNamingTopology(
      enrichDisplayLanguage(
        root,
        enrichUiSemanticChains(
          enrichSystemCoverageDepth(
            root,
            enrichLocalVerificationEvidence(
              root,
              enrichEvidenceMaturity(
                enrichApiCoverage(
                  root,
                  enrichSystemCoverage(root, compileSystemCoverage(root, options)),
                ),
              ),
            ),
          ),
        ),
      ),
    ),
  )
}
