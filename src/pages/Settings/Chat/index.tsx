import React from "react";
import { Switch } from "react-router-dom";

import { PrivateRoute } from "../../../components/PrivateRoute";
import ChatEmoteCreateForm from "./ChatEmoteCreateForm";
import ChatEmoteEditForm from "./ChatEmoteEditForm";
import ChatEmoteList from "./ChatEmoteList";
import ChatModifierCreateForm from "./ChatModifierCreateForm";
import ChatModifierEditForm from "./ChatModifierEditForm";
import ChatModifierList from "./ChatModifierList";
import ChatServerCreateForm from "./ChatServerCreateForm";
import ChatServerEditForm from "./ChatServerEditForm";
import ChatServerList from "./ChatServerList";
import ChatTagCreateForm from "./ChatTagCreateForm";
import ChatTagEditForm from "./ChatTagEditForm";
import ChatTagList from "./ChatTagList";

const Router: React.FC = () => (
  <main className="network_page">
    <Switch>
      <PrivateRoute path="/settings/chat-servers" exact component={ChatServerList} />
      <PrivateRoute path="/settings/chat-servers/new" exact component={ChatServerCreateForm} />
      <PrivateRoute path="/settings/chat-servers/:serverId" exact component={ChatServerEditForm} />
      <PrivateRoute
        path="/settings/chat-servers/:serverId/emotes"
        exact
        component={ChatEmoteList}
      />
      <PrivateRoute
        path="/settings/chat-servers/:serverId/emotes/new"
        exact
        component={ChatEmoteCreateForm}
      />
      <PrivateRoute
        path="/settings/chat-servers/:serverId/emotes/:emoteId"
        exact
        component={ChatEmoteEditForm}
      />
      <PrivateRoute
        path="/settings/chat-servers/:serverId/modifiers"
        exact
        component={ChatModifierList}
      />
      <PrivateRoute
        path="/settings/chat-servers/:serverId/modifiers/new"
        exact
        component={ChatModifierCreateForm}
      />
      <PrivateRoute
        path="/settings/chat-servers/:serverId/modifiers/:modifierId"
        exact
        component={ChatModifierEditForm}
      />
      <PrivateRoute path="/settings/chat-servers/:serverId/tags" exact component={ChatTagList} />
      <PrivateRoute
        path="/settings/chat-servers/:serverId/tags/new"
        exact
        component={ChatTagCreateForm}
      />
      <PrivateRoute
        path="/settings/chat-servers/:serverId/tags/:tagId"
        exact
        component={ChatTagEditForm}
      />
    </Switch>
  </main>
);

export default Router;
