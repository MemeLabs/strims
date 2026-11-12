// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Settings.stories.scss";

import { Base64 } from "js-base64";
import React, { ReactNode, useEffect, useRef } from "react";
import { Route } from "react-router-dom";

import Search from "../../../../components/Settings/Search";
import {
  CheckboxCell,
  CheckboxHeader,
  MenuCell,
  MenuHr,
  MenuItem,
  MenuLink,
  Table,
  TableCell,
  TableHeader,
  TableMenu,
  TableTitleBar,
} from "../../../../components/Settings/Table";
import { Story, StoryRouter, fixtures } from "../../../storybook";

const noop = (): void => undefined;

interface Row {
  id: bigint;
  name: string;
  key: string;
  peers: number;
  expires: string;
}

const rows = (count: number): Row[] =>
  Array.from({ length: count }, (_, i) => ({
    id: BigInt(i + 1),
    name: fixtures.networkName(i + 1),
    key: Base64.fromUint8Array(fixtures.networkKey(i + 1), true),
    peers: Math.pow(2, i % 12),
    expires: new Date(Number(fixtures.fixtureTime(30 + i)) * 1000).toISOString().substring(0, 10),
  }));

// The settings pages' tables sit in a StoryRouter: MenuLink and the title
// bar's back link are router links.
const Page: React.FC<{ children: ReactNode }> = ({ children }) => (
  <StoryRouter initialPath="/settings/networks">
    <Route path="/settings/networks" element={<div className="settings_story">{children}</div>} />
  </StoryRouter>
);

const RowMenu: React.FC = () => (
  <MenuCell>
    <MenuItem label="Create invite" onClick={noop} />
    <MenuItem label="Publish" onClick={noop} />
    <MenuLink label="Edit" to="/settings/networks/1" />
    <MenuHr />
    <MenuItem label="Delete" onClick={noop} />
  </MenuCell>
);

const CreateMenu: React.FC = () => (
  <TableMenu label="Add network">
    <MenuLink label="Create new network" to="/settings/networks/new" />
    <MenuLink label="Add invite code" to="/settings/networks/join" />
  </TableMenu>
);

interface NetworkTableProps {
  rows: Row[];
  checkboxes?: boolean;
}

const NetworkTable: React.FC<NetworkTableProps> = ({ rows, checkboxes }) => (
  <Table>
    <thead>
      <tr>
        {checkboxes && <CheckboxHeader name="networks" />}
        <TableHeader>Name</TableHeader>
        <TableHeader>Key</TableHeader>
        <TableHeader showFor="md">Peers</TableHeader>
        <TableHeader showFor="lg">Certificate expires</TableHeader>
        <TableHeader />
      </tr>
    </thead>
    <tbody>
      {rows.map((row) => (
        <tr key={row.id.toString()}>
          {checkboxes && <CheckboxCell name="networks" id={row.id} />}
          <TableCell>{row.name}</TableCell>
          <TableCell truncate>{row.key}</TableCell>
          <TableCell showFor="md">{row.peers}</TableCell>
          <TableCell showFor="lg">{row.expires}</TableCell>
          <RowMenu />
        </tr>
      ))}
    </tbody>
  </Table>
);

// Clicks the elements `selector` finds once mounted, as a user would: after
// the checkboxes' registrations have rendered, so the header's checked
// state counts them.
const ClickOnMount: React.FC<{ selector: string; children: ReactNode }> = ({
  selector,
  children,
}) => {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const tid = setTimeout(
      () => ref.current.querySelectorAll<HTMLElement>(selector).forEach((e) => e.click()),
      100
    );
    return () => clearTimeout(tid);
  }, []);
  return <div ref={ref}>{children}</div>;
};

const stories: Story[] = [
  {
    name: "Default",
    component: () => (
      <Page>
        <TableTitleBar label="Networks">
          <Search />
          <CreateMenu />
        </TableTitleBar>
        <NetworkTable rows={rows(5)} />
      </Page>
    ),
  },
  {
    name: "Back link",
    component: () => (
      <Page>
        <TableTitleBar label="Test Network peers" backLink="/settings/networks" />
        <NetworkTable rows={rows(3)} />
      </Page>
    ),
  },
  {
    name: "Empty",
    component: () => (
      <Page>
        <TableTitleBar label="Networks">
          <CreateMenu />
        </TableTitleBar>
        <NetworkTable rows={[]} />
      </Page>
    ),
  },
  {
    name: "Many rows",
    component: () => (
      <Page>
        <TableTitleBar label="Networks">
          <Search />
          <CreateMenu />
        </TableTitleBar>
        <NetworkTable rows={rows(30)} />
      </Page>
    ),
  },
  {
    name: "Truncate",
    component: () => (
      <Page>
        <div style={{ maxWidth: 420 }}>
          <Table>
            <thead>
              <tr>
                <TableHeader>Truncated</TableHeader>
                <TableHeader>Whole</TableHeader>
              </tr>
            </thead>
            <tbody>
              {rows(3).map((row) => (
                <tr key={row.id.toString()}>
                  <TableCell truncate>{row.key}</TableCell>
                  <TableCell>{row.name}</TableCell>
                </tr>
              ))}
            </tbody>
          </Table>
        </div>
      </Page>
    ),
  },
  {
    name: "Show for",
    component: () => (
      <Page>
        <p className="settings_story__caption">
          Columns marked sm, md, lg and xl show in frames at least 576, 768, 992 and 1200 px wide
          (the portrait frame shows none of them, landscape sm and md).
        </p>
        <Table>
          <thead>
            <tr>
              <TableHeader>always</TableHeader>
              <TableHeader showFor="sm">sm</TableHeader>
              <TableHeader showFor="md">md</TableHeader>
              <TableHeader showFor="lg">lg</TableHeader>
              <TableHeader showFor="xl">xl</TableHeader>
            </tr>
          </thead>
          <tbody>
            <tr>
              <TableCell>shown</TableCell>
              <TableCell showFor="sm">≥ 576 px</TableCell>
              <TableCell showFor="md">≥ 768 px</TableCell>
              <TableCell showFor="lg">≥ 992 px</TableCell>
              <TableCell showFor="xl">≥ 1200 px</TableCell>
            </tr>
          </tbody>
        </Table>
      </Page>
    ),
  },
  {
    name: "Checkboxes",
    component: () => (
      <Page>
        <ClickOnMount selector="tbody tr:nth-child(-n+2) input[type=checkbox]">
          <NetworkTable rows={rows(4)} checkboxes />
        </ClickOnMount>
      </Page>
    ),
  },
  {
    name: "All checked",
    component: () => (
      <Page>
        <ClickOnMount selector="thead input[type=checkbox]">
          <NetworkTable rows={rows(4)} checkboxes />
        </ClickOnMount>
      </Page>
    ),
  },
  {
    name: "Row menu",
    component: () => (
      <Page>
        <ClickOnMount selector="tbody tr:first-child .thing_table_item_dropdown__anchor">
          <NetworkTable rows={rows(4)} />
        </ClickOnMount>
      </Page>
    ),
  },
  {
    name: "Create menu",
    component: () => (
      <Page>
        <ClickOnMount selector=".thing_table_dropdown__anchor">
          <TableTitleBar label="Networks">
            <Search />
            <CreateMenu />
          </TableTitleBar>
          <NetworkTable rows={rows(3)} />
        </ClickOnMount>
      </Page>
    ),
  },
];

export default stories;
