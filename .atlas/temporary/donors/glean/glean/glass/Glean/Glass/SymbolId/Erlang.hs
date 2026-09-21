{-
  Copyright (c) Meta Platforms, Inc. and affiliates.
  All rights reserved.

  This source code is licensed under the BSD-style license found in the
  LICENSE file in the root directory of this source tree.
-}

{-# OPTIONS_GHC -Wno-orphans #-}

module Glean.Glass.SymbolId.Erlang
  ({- instances -})
  where

import TextShow

import Glean (Nat, fromNat, keyOf)
import Glean.Glass.SymbolId.Class
import Glean.Glass.Types (Name(..))
import qualified Glean.Haxl.Repos as Glean

import qualified Glean.Schema.Erlang.Types as Erlang
import Data.Text (Text, intercalate)

import Glean.Schema.CodeErlang.Types as CodeErlang
    ( Entity(..) )

appTok :: Text -> [Text]
appTok a = [a]

arityTok :: Maybe Nat -> Text
arityTok Nothing = "_"
arityTok (Just a) = showt (fromNat a)

instance Symbol CodeErlang.Entity where
  toSymbol e = case e of
    CodeErlang.Entity_decl decl -> toSymbol decl
    CodeErlang.Entity_macro_usage usage -> toSymbolPredicate usage
    CodeErlang.Entity_var_ var_ -> toSymbolPredicate var_
    CodeErlang.Entity_EMPTY -> return []

instance Symbol Erlang.Declaration where
  toSymbol decl = case decl of
    Erlang.Declaration_func func -> toSymbolPredicate func
    Erlang.Declaration_macro_ macro_ -> toSymbolPredicate macro_
    Erlang.Declaration_record_ record_ -> toSymbolPredicate record_
    Erlang.Declaration_type_ type_ -> toSymbolPredicate type_
    Erlang.Declaration_header_ header_ -> toSymbolPredicate header_
    Erlang.Declaration_callback_ callback_ -> toSymbolPredicate callback_
    Erlang.Declaration_record_field rf -> toSymbolPredicate rf
    Erlang.Declaration_module_ mod_ -> toSymbolPredicate mod_
    Erlang.Declaration_EMPTY -> return []

instance Symbol Erlang.FunctionDeclaration_key where
  toSymbol (Erlang.FunctionDeclaration_key fqn app) =
    do base <- toSymbol fqn
       return $ ["func"] ++ appTok app ++ base

instance Symbol Erlang.MacroDeclaration_key where
  toSymbol (Erlang.MacroDeclaration_key name arity module_ app) =
    return $ ["define"] ++ appTok app ++ [module_, name, arityTok arity]

instance Symbol Erlang.RecordDeclaration_key where
  toSymbol (Erlang.RecordDeclaration_key name module_ app) =
    return $ ["record"] ++ appTok app ++ [module_, name]

instance Symbol Erlang.TypeDeclaration_key where
  toSymbol (Erlang.TypeDeclaration_key name arity module_ app) =
    return $ ["type"] ++ appTok app ++ [module_, name, showt (fromNat arity)]

instance Symbol Erlang.HeaderDeclaration_key where
  toSymbol (Erlang.HeaderDeclaration_key name app) =
    return $ ["header"] ++ appTok app ++ [name]

instance Symbol Erlang.CallbackDeclaration_key where
  toSymbol (Erlang.CallbackDeclaration_key name arity module_ app) =
    return $ ["callback"] ++ appTok app ++
      [module_, name, showt (fromNat arity)]

instance Symbol Erlang.RecordFieldDeclaration_key where
  toSymbol (Erlang.RecordFieldDeclaration_key recName fieldName module_ app) =
    return $ ["record_field"] ++ appTok app ++ [module_, recName, fieldName]

instance Symbol Erlang.ModuleDeclaration_key where
  toSymbol (Erlang.ModuleDeclaration_key _file name app) =
    return $ ["module"] ++ appTok app ++ [name]

instance Symbol Erlang.MacroUsage_key where
  toSymbol (Erlang.MacroUsage_key name defModule arity app
    _expansion _links contentHash) =
    return $ ["macro_usage"] ++ appTok app
      ++ [defModule, name, arityTok arity, contentHash]

instance Symbol Erlang.VarDeclaration_key where
  toSymbol (Erlang.VarDeclaration_key name module_ app spanStart _typeText) =
    return $ ["var"] ++ appTok app ++ [module_, name, showt (fromNat spanStart)]

instance Symbol Erlang.Fqn where
  toSymbol (Erlang.Fqn module_ name arity) =
    return [module_, name, showt (fromNat arity)]

instance Symbol Text where
  toSymbol module_ = return [module_]

instance Symbol (Text, Nat) where
  toSymbol (name, arity) =
      return [intercalate "." [name, showt (fromNat arity)]]

instance ToQName CodeErlang.Entity where
  toQName e = case e of
    CodeErlang.Entity_decl x -> toQName x
    CodeErlang.Entity_macro_usage x -> Glean.keyOf x >>= toQName
    CodeErlang.Entity_var_ x -> Glean.keyOf x >>= toQName
    CodeErlang.Entity_EMPTY -> return $ Left "unknown Entity"

instance ToQName Erlang.Declaration where
  toQName e = case e of
    Erlang.Declaration_func x -> Glean.keyOf x >>= toQName
    Erlang.Declaration_macro_ x -> Glean.keyOf x >>= toQName
    Erlang.Declaration_record_ x -> Glean.keyOf x >>= toQName
    Erlang.Declaration_type_ x -> Glean.keyOf x >>= toQName
    Erlang.Declaration_header_ x -> Glean.keyOf x >>= toQName
    Erlang.Declaration_callback_ x -> Glean.keyOf x >>= toQName
    Erlang.Declaration_record_field x -> Glean.keyOf x >>= toQName
    Erlang.Declaration_module_ x -> Glean.keyOf x >>= toQName
    Erlang.Declaration_EMPTY -> return $ Left "unknown Declaration"

instance ToQName Erlang.FunctionDeclaration_key where
  toQName (Erlang.FunctionDeclaration_key fqn _app) = toQName fqn

instance ToQName Erlang.MacroDeclaration_key where
  toQName (Erlang.MacroDeclaration_key name _arity module_ _app) =
    return $ Right (Name name, Name module_)

instance ToQName Erlang.RecordDeclaration_key where
  toQName (Erlang.RecordDeclaration_key name module_ _app) =
    return $ Right (Name name, Name module_)

instance ToQName Erlang.TypeDeclaration_key where
  toQName (Erlang.TypeDeclaration_key name _arity module_ _app) =
    return $ Right (Name name, Name module_)

instance ToQName Erlang.HeaderDeclaration_key where
  toQName (Erlang.HeaderDeclaration_key name _app) =
    return $ Right (Name name, Name name)

instance ToQName Erlang.CallbackDeclaration_key where
  toQName (Erlang.CallbackDeclaration_key name _arity module_ _app) =
    return $ Right (Name name, Name module_)

instance ToQName Erlang.RecordFieldDeclaration_key where
  toQName (Erlang.RecordFieldDeclaration_key _recName fieldName module_ _app) =
    return $ Right (Name fieldName, Name module_)

instance ToQName Erlang.ModuleDeclaration_key where
  toQName (Erlang.ModuleDeclaration_key _file name _app) =
    return $ Right (Name name, Name name)

instance ToQName Erlang.MacroUsage_key where
  toQName (Erlang.MacroUsage_key name defModule _arity _app
    _expansion _links _contentHash) =
    return $ Right (Name name, Name defModule)

instance ToQName Erlang.VarDeclaration_key where
  toQName (Erlang.VarDeclaration_key name module_ _app _spanStart _typeText) =
    return $ Right (Name name, Name module_)

instance ToQName Erlang.Fqn where
  toQName (Erlang.Fqn module_ name arity) =
    pairToQName (name, arity) module_

pairToQName
  :: (Symbol name, Symbol container)
  => name
  -> container
  -> Glean.RepoHaxl u w (Either a (Name, Name))
pairToQName a b = Right <$> symbolPairToQName "." a b
