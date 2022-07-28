// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./InviteAuth.scss";

import { Base64 } from "js-base64";
import { isEqual } from "lodash";
import React from "react";
import { Control, useForm } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { Navigate, useNavigate } from "react-router";
import { useAsync } from "react-use";

import { InviteClient } from "../apis/client";
import { BootstrapClient } from "../apis/strims/network/v1/bootstrap/bootstrap";
import { Invitation, InvitationV0 } from "../apis/strims/network/v1/network";
import { Profile } from "../apis/strims/profile/v1/profile";
import { Certificate } from "../apis/strims/type/certificate";
import { useCall, useLazyCall } from "../contexts/FrontendApi";
import { useSession } from "../contexts/Session";
import { suppressClickAway } from "../hooks/useClickAway";
import { certificateChain } from "../lib/certificate";
import { HTTPReadWriter } from "../lib/http";
import { networkKey } from "../lib/network";
import { validNamePattern } from "../lib/validation";
import { Button, InputError, InputLabel, TextInput, ToggleInput } from "./Form";
import InternalLink from "./InternalLink";

interface InviteAuthProps {
  code: string;
}

const InviteAuth: React.FC<InviteAuthProps> = ({ code }) => {
  const inviteRes = useAsync(async () => {
    const rw = new HTTPReadWriter("/api/invite");
    const client = new InviteClient(rw, rw);

    const res = await Promise.race([
      client.inviteLink.getInvitation({ code }),
      new Promise<never>((_, reject) =>
        setTimeout(() => reject(new Error("request timeout")), 1000)
      ),
    ]);
    if (res.invitation.version !== 0) {
      throw new Error("unexpected invitation version");
    }
    const invitation = InvitationV0.decode(res.invitation.data);
    const [networkCert, peerCert] = certificateChain(invitation.certificate);
    return { invitation, networkCert, peerCert };
  }, [code]);

  const { t } = useTranslation();
  const [{ profile }] = useSession();
  const [networksRes] = useCall("network", "list");

  if (inviteRes.error || networksRes.error) {
    return (
      <InviteAuthModal>
        <div className="invite_auth__error">{t("inviteAuth.error")}</div>
        <InternalLink to="/">{t("inviteAuth.Continue")}</InternalLink>
      </InviteAuthModal>
    );
  }
  if (!inviteRes.value || !networksRes.value) {
    return <InviteAuthModal>{t("inviteAuth.Loading")}</InviteAuthModal>;
  }

  const { networkCert } = inviteRes.value;
  if (networksRes.value.networks.some((n) => isEqual(networkKey(n), networkCert.key))) {
    return <Navigate to={`/directory/${Base64.fromUint8Array(networkCert.key, true)}`} />;
  }

  return (
    <InviteAuthModal>
      <InviteAuthForm {...inviteRes.value} profile={profile} />
    </InviteAuthModal>
  );
};

interface InviteAuthModalProps {
  children: React.ReactNode;
}

const InviteAuthModal: React.FC<InviteAuthModalProps> = ({ children }) => (
  <div className="invite_auth" {...suppressClickAway()}>
    <div className="invite_auth__mask" />
    <div className="invite_auth__modal">{children}</div>
  </div>
);

export default InviteAuth;

interface InviteAuthFormValues {
  alias: string;
  enabledBootstrapClients: boolean[];
  [key: string]: any;
}

interface InviteAuthFormProps {
  invitation: InvitationV0;
  networkCert: Certificate;
  peerCert: Certificate;
  profile: Profile;
}

const InviteAuthForm: React.FC<InviteAuthFormProps> = ({
  invitation,
  networkCert,
  peerCert,
  profile,
}) => {
  const { t } = useTranslation();
  const navigate = useNavigate();

  const { bootstrapClients } = invitation;

  const { control, handleSubmit, formState } = useForm<InviteAuthFormValues>({
    mode: "onBlur",
    defaultValues: {
      alias: profile.name,
      enabledBootstrapClients: bootstrapClients.map(() => true),
    },
  });

  const [{ error }, create] = useLazyCall("network", "createNetworkFromInvitation", {
    onComplete: () => navigate(`/directory/${Base64.fromUint8Array(networkCert.key, true)}`),
  });

  const onSubmit = handleSubmit(({ alias, enabledBootstrapClients }) =>
    create({
      alias,
      invitation: {
        invitationBytes: Invitation.encode(
          new Invitation({
            version: 0,
            data: InvitationV0.encode(
              new InvitationV0({
                key: invitation.key,
                certificate: invitation.certificate,
                bootstrapClients: invitation.bootstrapClients.filter(
                  (_, i) => enabledBootstrapClients[i]
                ),
              })
            ).finish(),
          })
        ).finish(),
      },
    })
  );

  return (
    <form onSubmit={onSubmit}>
      <span className="invite_auth__peer">
        {t("inviteAuth.header", { peer: peerCert.subject })}
      </span>
      <span className="invite_auth__network_name">{networkCert.subject}</span>

      {error && <InputError error={error.message || t("inviteAuth.Error creating membership")} />}
      <TextInput
        control={control}
        rules={{
          pattern: {
            value: validNamePattern,
            message: t("inviteAuth.Name contains invalid characters"),
          },
        }}
        label={t("inviteAuth.Alias")}
        name="alias"
        placeholder={t("inviteAuth.Enter an alternate name for this network")}
      />
      {bootstrapClients.length > 0 && (
        <InputLabel
          text={t("inviteAuth.bootstrapServers", { count: bootstrapClients.length })}
          component="div"
        >
          {bootstrapClients.map((client, i) => (
            <BootstrapClientCheckbox key={i} client={client} index={i} control={control} />
          ))}
        </InputLabel>
      )}
      <Button className="invite_auth--primary" disabled={formState.isSubmitting}>
        {t("inviteAuth.Continue")}
      </Button>
      <InternalLink className="invite_auth__reject" to="/">
        {t("inviteAuth.Reject invitation")}
      </InternalLink>
    </form>
  );
};

interface BootstrapClientCheckboxProps {
  client: BootstrapClient;
  index: number;
  control: Control<InviteAuthFormValues, object>;
}

const BootstrapClientCheckbox: React.FC<BootstrapClientCheckboxProps> = ({
  client,
  index,
  control,
}) => {
  switch (client.clientOptions.case) {
    case BootstrapClient.ClientOptionsCase.WEBSOCKET_OPTIONS: {
      return (
        <ToggleInput
          control={control}
          label={new URL(client.clientOptions.websocketOptions.url).host}
          name={`enabledBootstrapClients.${index}`}
        />
      );
    }
    default:
      return null;
  }
};
