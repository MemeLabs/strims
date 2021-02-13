import React from "react";

const LandingPageLayout: React.FC = ({ children }) => (
  <div className="landing_page">
    <div className="landing_page__header">
      <h1 className="landing_page__header__title">strims@home</h1>
      <span className="landing_page__header__tagline">Watch strims with frens.</span>
    </div>
    <div className="landing_page__form_container">{children}</div>
  </div>
);

export default LandingPageLayout;
