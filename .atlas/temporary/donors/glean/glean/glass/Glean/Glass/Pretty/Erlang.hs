{-
  Copyright (c) Meta Platforms, Inc. and affiliates.
  All rights reserved.

  This source code is licensed under the BSD-style license found in the
  LICENSE file in the root directory of this source tree.
-}

{-# LANGUAGE TypeApplications #-}

module Glean.Glass.Pretty.Erlang
  ( prettyErlangSignature
  , prettyFuncDecl
  , prettyRecordDecl
  , prettyTypeDecl
  , prettyMacroShortDecl
  , prettyHeaderDecl
  , prettyCallbackDecl
  , prettyRecordFieldDecl
  , prettyModuleDecl
  ) where

import Data.Maybe (fromMaybe)
import qualified Data.Text as Text
import Data.Text ( Text )
import Compat.Prettyprinter

import Glean (keyOf, fromNat)
import qualified Glean.Haxl.Repos as Glean

import Glean.Angle as Angle
import Glean.Schema.CodeErlang.Types as CodeErlang ( Entity(..) )
import qualified Glean.Schema.Erlang.Types as Erlang

import Glean.Glass.Types ( SymbolId )
import Glean.Glass.Utils ( searchWithLimit )

prettyErlangSignature
  :: LayoutOptions
  -> CodeErlang.Entity
  -> Glean.RepoHaxl u w (Maybe (SimpleDocStream (Maybe SymbolId)))
prettyErlangSignature opts entity = case entity of
  CodeErlang.Entity_decl decl -> prettyDecl opts decl
  CodeErlang.Entity_macro_usage usage -> prettyMacroUsage opts usage
  CodeErlang.Entity_var_ var_ -> do
    key <- keyOf var_
    return $ Just $ layoutDoc opts $ prettyVarDecl key
  CodeErlang.Entity_EMPTY -> return Nothing

prettyDecl
  :: LayoutOptions
  -> Erlang.Declaration
  -> Glean.RepoHaxl u w (Maybe (SimpleDocStream (Maybe SymbolId)))
prettyDecl opts decl = case decl of
  Erlang.Declaration_func func -> do
    key <- keyOf func
    return $ Just $ layoutDoc opts $ prettyFuncDecl key
  Erlang.Declaration_macro_ macro_ -> do
    key <- keyOf macro_
    return $ Just $ layoutDoc opts $ prettyMacroShortDecl key
  Erlang.Declaration_record_ record_ -> do
    key <- keyOf record_
    return $ Just $ layoutDoc opts $ prettyRecordDecl key
  Erlang.Declaration_type_ type_ -> do
    key <- keyOf type_
    return $ Just $ layoutDoc opts $ prettyTypeDecl key
  Erlang.Declaration_header_ header_ -> do
    key <- keyOf header_
    return $ Just $ layoutDoc opts $ prettyHeaderDecl key
  Erlang.Declaration_callback_ callback_ -> do
    key <- keyOf callback_
    return $ Just $ layoutDoc opts $ prettyCallbackDecl key
  Erlang.Declaration_record_field rf -> do
    key <- keyOf rf
    return $ Just $ layoutDoc opts $ prettyRecordFieldDecl key
  Erlang.Declaration_module_ mod_ -> do
    key <- keyOf mod_
    return $ Just $ layoutDoc opts $ prettyModuleDecl key
  Erlang.Declaration_EMPTY -> return Nothing

prettyMacroUsage
  :: LayoutOptions
  -> Erlang.MacroUsage
  -> Glean.RepoHaxl u w (Maybe (SimpleDocStream (Maybe SymbolId)))
prettyMacroUsage opts usage = do
  Erlang.MacroUsage_key macroName macroModule _macroArity _app
    _expansion _links _contentHash <- keyOf usage
  allDefs <- searchWithLimit (Just 10) $
    Angle.predicate @Erlang.MacroDefinition $
      rec $
        field @"declaration" (rec $
          field @"name" (string macroName) $
          field @"module" (string macroModule)
        end)
      end
  defText <- do
    keys <- mapM keyOf allDefs
    return $ case keys of
      (Erlang.MacroDefinition_key _ dt : _) -> dt
      _ -> Nothing
  let doc = case defText of
        Just t -> pretty (stripCodeFences t)
        Nothing -> "-define(" <> pretty macroName <> ", ...)"
  return $ Just $ layoutDoc opts doc

layoutDoc :: LayoutOptions -> Doc ann -> SimpleDocStream ann
layoutDoc opts doc = layoutSmart opts doc

prettyFuncDecl :: Erlang.FunctionDeclaration_key -> Doc (Maybe SymbolId)
prettyFuncDecl (Erlang.FunctionDeclaration_key fqn _app) =
  let Erlang.Fqn module_ name arity = fqn in
  pretty module_ <> ":" <> pretty name <> "/" <> pretty (show (fromNat arity))

prettyRecordDecl :: Erlang.RecordDeclaration_key -> Doc (Maybe SymbolId)
prettyRecordDecl (Erlang.RecordDeclaration_key name module_ _app) =
  "-record(" <> pretty name <> ", {...})" <+> pretty ("in " <> module_)

prettyTypeDecl :: Erlang.TypeDeclaration_key -> Doc (Maybe SymbolId)
prettyTypeDecl (Erlang.TypeDeclaration_key name arity module_ _app) =
  "-type " <> pretty name <> "/" <> pretty (show (fromNat arity))
    <+> pretty ("in " <> module_)

prettyMacroShortDecl :: Erlang.MacroDeclaration_key -> Doc (Maybe SymbolId)
prettyMacroShortDecl (Erlang.MacroDeclaration_key name arity module_ _app) =
  "?" <> pretty name <> prettyArity arity <+> pretty ("in " <> module_)
  where
    prettyArity Nothing = mempty
    prettyArity (Just a) = "/" <> pretty (show (fromNat a))

prettyHeaderDecl :: Erlang.HeaderDeclaration_key -> Doc (Maybe SymbolId)
prettyHeaderDecl (Erlang.HeaderDeclaration_key name _app) =
  "-include(\"" <> pretty name <> "\")"

prettyCallbackDecl :: Erlang.CallbackDeclaration_key -> Doc (Maybe SymbolId)
prettyCallbackDecl (Erlang.CallbackDeclaration_key name arity _module _app) =
  "-callback " <> pretty name <> "/" <> pretty (show (fromNat arity))

prettyRecordFieldDecl ::
  Erlang.RecordFieldDeclaration_key -> Doc (Maybe SymbolId)
prettyRecordFieldDecl
  (Erlang.RecordFieldDeclaration_key recName fieldName _module _app) =
  "#" <> pretty recName <> "." <> pretty fieldName

prettyModuleDecl :: Erlang.ModuleDeclaration_key -> Doc (Maybe SymbolId)
prettyModuleDecl (Erlang.ModuleDeclaration_key _file name _app) =
  "-module(" <> pretty name <> ")."

prettyVarDecl :: Erlang.VarDeclaration_key -> Doc (Maybe SymbolId)
prettyVarDecl
  (Erlang.VarDeclaration_key name _module _app _spanStart typeText) =
  case typeText of
    Just t -> pretty (stripCodeFences t)
    Nothing -> pretty name

stripCodeFences :: Text -> Text
stripCodeFences t =
  let stripped = Text.strip t
      withoutOpen =
        fromMaybe
          (fromMaybe stripped (Text.stripPrefix "```\n" stripped))
          (Text.stripPrefix "```erlang\n" stripped)
      withoutClose = fromMaybe withoutOpen $
        Text.stripSuffix "\n```" withoutOpen
  in Text.strip withoutClose
