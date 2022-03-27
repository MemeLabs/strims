import React from "react";
import { FiUser } from "react-icons/fi";
import { Link, Navigate, useLocation } from "react-router-dom";

import { LinkedProfile } from "../apis/strims/auth/v1/auth";
import ProfileForm, { ProfileFormValues } from "../components/Landing/ProfileForm";
import LandingPageLayout from "../components/LandingPageLayout";
import LoadingPlaceholder from "../components/LoadingPlaceholder";
import { useSession } from "../contexts/Session";
import useQuery from "../hooks/useQuery";
import useReady from "../hooks/useReady";

const VALID_NEXT_PATH = /^\/\w[\w/_\-.?=#%&]*$/;

interface LinkedProfileListItemProps {
  profile: LinkedProfile;
  onClick: (profile: LinkedProfile) => void;
}

const LinkedProfileListItem: React.FC<LinkedProfileListItemProps> = ({ profile, onClick }) => {
  let address = null;
  if (profile.serverAddress) {
    const url = new URL(profile.serverAddress);
    address = <span className="login_profile_list__subtext">{url.hostname}</span>;
  }

  return (
    <div
      className="login_profile_list__item"
      key={profile.id.toString()}
      onClick={() => onClick(profile)}
    >
      <FiUser className="login_profile_list__icon" />
      <span className="login_profile_list__label">
        <span className="login_profile_list__text">{profile.name}</span>
        {address}
      </span>
    </div>
  );
};

interface LoginQueryParams {
  next: string;
}

const LoginPage: React.FC = () => {
  const [selectedProfile, setSelectedProfile] = React.useState<LinkedProfile | null>(null);
  const [session, sessionOps] = useSession();
  const { next } = useQuery<LoginQueryParams>(useLocation().search);

  useReady(() => {
    const { credentials } = selectedProfile;
    switch (credentials.case) {
      case LinkedProfile.CredentialsCase.UNENCRYPTED:
        // TODO
        break;
      case LinkedProfile.CredentialsCase.PASSWORD:
        // TODO
        break;
      case LinkedProfile.CredentialsCase.TOKEN:
        void sessionOps.signIn(selectedProfile.serverAddress, {
          credentials: { token: credentials.token },
        });
        break;
      case LinkedProfile.CredentialsCase.KEY:
        void sessionOps.signIn(selectedProfile.serverAddress, {
          credentials: { key: credentials.key },
        });
        break;
    }
  }, [selectedProfile]);

  if (session.loading) {
    return <LoadingPlaceholder />;
  }
  if (session.profile) {
    return <Navigate to={VALID_NEXT_PATH.test(next) ? next : "/"} />;
  }

  if (!selectedProfile && session.linkedProfiles.length) {
    return (
      <LandingPageLayout>
        <div className="login_profile_list">
          {session.linkedProfiles.map((profile) => (
            <LinkedProfileListItem
              key={profile.id.toString()}
              profile={profile}
              onClick={setSelectedProfile}
            />
          ))}
          <Link className="input input_button input_button--borderless" to="/signup">
            Create Profile
          </Link>
          <button
            className="input input_button input_button--borderless"
            onClick={() => setSelectedProfile(new LinkedProfile())}
          >
            New Login
          </button>
        </div>
      </LandingPageLayout>
    );
  }

  const handleSubmit = (values: ProfileFormValues) => {
    void sessionOps.signIn(values.serverAddress, {
      credentials: {
        password: {
          name: values.name,
          password: values.password,
          persistLogin: values.persistLogin,
        },
      },
    });
  };

  return (
    <LandingPageLayout>
      <ProfileForm
        onSubmit={handleSubmit}
        error={session.error?.message}
        secondaryUri="/signup"
        secondaryLabel="Create profile"
        submitLabel="Log in"
        defaultValues={{
          name: selectedProfile?.name,
          serverAddress: selectedProfile?.serverAddress,
        }}
      />
    </LandingPageLayout>
  );
};

export default LoginPage;
