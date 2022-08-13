// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { Route, Routes } from "react-router-dom";

import ChatEmoteCreateForm from "./ChatEmoteCreateForm";
import ChatEmoteEditForm from "./ChatEmoteEditForm";
import ChatEmoteList from "./ChatEmoteList";
import ChatModifierCreateForm from "./ChatModifierCreateForm";
import ChatModifierEditForm from "./ChatModifierEditForm";
import ChatModifierList from "./ChatModifierList";
import ChatServerCreateForm from "./ChatServerCreateForm";
import ChatServerEditForm from "./ChatServerEditForm";
import ChatServerIconEditForm from "./ChatServerIconEditForm";
import ChatServerList from "./ChatServerList";
import ChatTagCreateForm from "./ChatTagCreateForm";
import ChatTagEditForm from "./ChatTagEditForm";
import ChatTagList from "./ChatTagList";

const Router: React.FC = () => (
  <Routes>
    <Route index element={<ChatServerList />} />
    <Route path="new" element={<ChatServerCreateForm />} />
    <Route path=":serverId" element={<ChatServerEditForm />} />
    <Route path=":serverId/icon" element={<ChatServerIconEditForm />} />
    <Route path=":serverId/emotes" element={<ChatEmoteList />} />
    <Route path=":serverId/emotes/new" element={<ChatEmoteCreateForm />} />
    <Route path=":serverId/emotes/:emoteId" element={<ChatEmoteEditForm />} />
    <Route path=":serverId/modifiers" element={<ChatModifierList />} />
    <Route path=":serverId/modifiers/new" element={<ChatModifierCreateForm />} />
    <Route path=":serverId/modifiers/:modifierId" element={<ChatModifierEditForm />} />
    <Route path=":serverId/tags" element={<ChatTagList />} />
    <Route path=":serverId/tags/new" element={<ChatTagCreateForm />} />
    <Route path=":serverId/tags/:tagId" element={<ChatTagEditForm />} />
  </Routes>
);

export default Router;
