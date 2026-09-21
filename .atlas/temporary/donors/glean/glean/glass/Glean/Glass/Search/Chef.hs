{-
  Copyright (c) Meta Platforms, Inc. and affiliates.
  All rights reserved.

  This source code is licensed under the BSD-style license found in the
  LICENSE file in the root directory of this source tree.
-}

{-# LANGUAGE TypeApplications #-}
{-# OPTIONS_GHC -Wno-orphans #-}

module Glean.Glass.Search.Chef
  ( {- instances -}
  ) where

import Data.Text ( Text )

import Glean.Angle as Angle

import Glean.Glass.Search.Class
import Glean.Glass.Query ( entityLocation )

import qualified Glean.Schema.CodeChef.Types as CodeChef
import qualified Glean.Schema.CodemarkupTypes.Types as Code
import qualified Glean.Schema.SearchChef.Types as SearchChef
import qualified Glean.Schema.Src.Types as Src

--
-- Recover Chef entities from their Symbol ID encoding.
--
-- Chef symbol IDs encode `qualifiedName ++ [identifier]` (see
-- Glean.Glass.SymbolId.Chef). The last token is the leaf identifier; we
-- use the derived `search.chef.SearchByName` predicate to look up the
-- corresponding `code.chef.Entity` by that identifier.
--

instance Search (ResultLocation CodeChef.Entity) where
  symbolSearch toks = case reverse toks of
    (name : _) -> searchSymbolId toks $ searchByName name
    _ -> return $ None "Chef.symbolSearch: empty symbol id"

searchByName :: Text -> Angle (ResultLocation CodeChef.Entity)
searchByName name =
  vars $ \(ent :: Angle CodeChef.Entity) (file :: Angle Src.File)
    (rangespan :: Angle Code.RangeSpan) (lname :: Angle Text) ->
  tuple (ent, file, rangespan, lname) `where_` [
    wild .= predicate @SearchChef.SearchByName (
      rec $
        field @"name" (string name) $
        field @"entity" ent
      end),
    entityLocation (alt @"chef" ent) file rangespan lname
  ]
