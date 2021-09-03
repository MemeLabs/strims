import clsx from "clsx";
import { Base64 } from "js-base64";
import React, { useCallback, useRef, useState } from "react";
import Scrollbars from "react-custom-scrollbars-2";
import { Controller, useForm } from "react-hook-form";
import { BiConversation, BiSmile } from "react-icons/bi";
import { BsArrowBarLeft, BsArrowBarRight } from "react-icons/bs";
import { FiSettings } from "react-icons/fi";
import { HiOutlineUserGroup } from "react-icons/hi";
import { ImCircleLeft, ImCircleRight } from "react-icons/im";
import { IconType } from "react-icons/lib";
import Select from "react-select";
import CreatableSelect from "react-select/creatable";
import { useToggle } from "react-use";
import { useClickOutside } from "use-events";

import Composer from "../components/Chat/Composer";
import Message from "../components/Chat/Message";
import Scroller, { MessageProps } from "../components/Chat/Scroller";
import { InputError, InputLabel, TextInput, ToggleInput } from "../components/Form";
import { Provider, useChat } from "../contexts/Chat";
import Emote from "./Chat/Emote";

const TestEmotes: React.FC = () => {
  const [chat] = useChat();

  return (
    <Scrollbars autoHide={true}>
      <div className="chat__emote_grid">
        {chat.emotes.map((name) => (
          <div key={name} className="chat__emote_grid__emote">
            <Emote name={name} />
          </div>
        ))}
      </div>
    </Scrollbars>
  );
};

interface SettingsFormData {
  showTime: boolean;
  showFlairIcons: boolean;
  timestampFormat: string;
  maxLines: number;
  notificationWhisper: boolean;
  soundNotificationWhisper: boolean;
  notificationHighlight: boolean;
  soundNotificationHighlight: boolean;
  notificationSoundFile: {
    fileType: string;
    data: string;
  };
  highlight: boolean;
  customHighlight: string[];
  highlightNicks: string[];
  taggedNicks: string[];
  showRemoved: {
    value: string;
    label: string;
  };
  showWhispersInChat: boolean;
  ignoreNicks: string[];
  focusMentioned: boolean;
  notificationTimeout: boolean;
  ignoreMentions: boolean;
  autocompleteHelper: boolean;
  autocompleteEmotePreview: boolean;
  taggedVisibility: boolean;
  hideNsfw: boolean;
  animateForever: boolean;
  formatterGreen: boolean;
  formatterEmote: boolean;
  formatterCombo: boolean;
  holidayEmoteModifiers: boolean;
  disableSpoilers: boolean;
  viewerStateIndicator: {
    value: string;
    label: string;
  };
  hiddenEmotes: string[];
}

const stateIndicatorOptions = [
  {
    value: "disable",
    label: "Disable",
  },
  {
    value: "bar",
    label: "Bar",
  },
  {
    value: "dot",
    label: "Dot",
  },
  {
    value: "array",
    label: "Array",
  },
];

const showRemovedOptions = [
  {
    value: "remove",
    label: "Remove",
  },
  {
    value: "censor",
    label: "Censor",
  },
  {
    value: "do nothing",
    label: "Do nothing",
  },
];

const TestSettings: React.FC = () => {
  // const [chat] = useChat();

  const { control } = useForm<SettingsFormData>({
    mode: "onBlur",
    defaultValues: {
      showTime: false,
      showFlairIcons: true,
      timestampFormat: "HH:mm",
      maxLines: 250,
      notificationWhisper: true,
      soundNotificationWhisper: false,
      notificationHighlight: true,
      soundNotificationHighlight: false,
      highlight: true,
      customHighlight: [],
      highlightNicks: [],
      taggedNicks: [],
      showRemoved: showRemovedOptions[0],
      showWhispersInChat: true,
      ignoreNicks: [],
      focusMentioned: false,
      notificationTimeout: true,
      ignoreMentions: false,
      autocompleteHelper: true,
      autocompleteEmotePreview: true,
      taggedVisibility: false,
      hideNsfw: false,
      animateForever: true,
      formatterGreen: true,
      formatterEmote: true,
      formatterCombo: true,
      holidayEmoteModifiers: true,
      disableSpoilers: false,
      viewerStateIndicator: stateIndicatorOptions[1],
      hiddenEmotes: [],
    },
  });

  return (
    <Scrollbars autoHide={true}>
      <div className="chat__settings">
        <form className="chat__settings__form">
          {/* {error && <InputError error={error.message || "Error creating chat server"} />} */}
          <fieldset>
            <legend>Messages</legend>
            <ToggleInput control={control} label="Show flair" name="showFlairIcons" />
            <ToggleInput control={control} label="Show time" name="showTime" />
            <ToggleInput control={control} label="Harsh ignore" name="ignoreMentions" />
            <ToggleInput
              control={control}
              label="Hide messages tagged nsfw or nsfl"
              name="hideNsfw"
            />
            <ToggleInput
              control={control}
              label="Loop animated emotes forever"
              name="animateForever"
            />
            <ToggleInput control={control} label="Disable spoilers" name="disableSpoilers" />
            <TextInput
              control={control}
              label="Maximum messages"
              name="maxLines"
              type="number"
              rules={{
                pattern: {
                  value: /^\d+$/i,
                  message: "Maximum messages must be numeric",
                },
              }}
            />
            <InputLabel text="Stream viewer indicators">
              <Controller
                name="viewerStateIndicator"
                control={control}
                render={({ field, fieldState: { error } }) => (
                  <>
                    <Select
                      {...field}
                      className="input_select"
                      classNamePrefix="react_select"
                      options={stateIndicatorOptions}
                    />
                    <InputError error={error} />
                  </>
                )}
              />
            </InputLabel>
            <InputLabel text="Banned messages">
              <Controller
                name="showRemoved"
                control={control}
                render={({ field, fieldState: { error } }) => (
                  <>
                    <Select
                      {...field}
                      className="input_select"
                      classNamePrefix="react_select"
                      options={showRemovedOptions}
                    />
                    <InputError error={error} />
                  </>
                )}
              />
            </InputLabel>
          </fieldset>
          <fieldset>
            <legend>Autocomplete</legend>
            <ToggleInput control={control} label="Auto-complete helper" name="autocompleteHelper" />
            <ToggleInput
              control={control}
              label="Show emote preview"
              name="autocompleteEmotePreview"
            />
          </fieldset>
          <fieldset>
            <legend>Whispers</legend>
            <ToggleInput control={control} label="In-line messages" name="showWhispersInChat" />
          </fieldset>

          <fieldset>
            <legend>Highlights, focus &amp; tags</legend>
            <ToggleInput control={control} label="Highlight when mentioned" name="highlight" />
            <ToggleInput
              control={control}
              label="Include mentions when focused"
              name="ignoreMentions"
            />
            <ToggleInput
              control={control}
              label="Increase visibility of tagged users"
              name="taggedVisibility"
            />
            <InputLabel text="Custom highlights">
              <Controller
                name="customHighlight"
                control={control}
                render={({ field, fieldState: { error } }) => (
                  <>
                    <CreatableSelect
                      {...field}
                      isMulti={true}
                      placeholder="Custom highlights"
                      className="input_select"
                      classNamePrefix="react_select"
                    />
                    <InputError error={error} />
                  </>
                )}
              />
            </InputLabel>
          </fieldset>
          <fieldset>
            <legend>Autocomplete</legend>
            <ToggleInput control={control} label="Autocomplete helper" name="autocompleteHelper" />
            <ToggleInput
              control={control}
              label="Show emote previews"
              name="autocompleteEmotePreview"
            />
          </fieldset>
          <fieldset>
            <legend>Message formatters</legend>
            <ToggleInput control={control} label="Greentext" name="formatterGreen" />
            <ToggleInput control={control} label="Emotes" name="formatterEmote" />
            <ToggleInput control={control} label="Combos" name="formatterCombo" />
            <ToggleInput control={control} label="Modifiers" name="holidayEmoteModifiers" />
          </fieldset>
        </form>
      </div>
    </Scrollbars>
  );
};

enum ChatDrawerRole {
  None = "none",
  Emotes = "emotes",
  Whispers = "whispers",
  Settings = "settings",
  Users = "users",
}

interface ChatDrawerBodyProps {
  onClose: () => void;
}

const ChatDrawerBody: React.FC<ChatDrawerBodyProps> = ({ onClose, children }) => {
  const ref = useRef<HTMLDivElement>(null);
  useClickOutside([ref], onClose);

  return (
    <div className="chat__drawer__body" ref={ref}>
      {children}
    </div>
  );
};

interface ChatDrawerProps extends ChatDrawerBodyProps {
  side: "left" | "right";
  role: ChatDrawerRole;
  title: string;
  active: boolean;
}

const ChatDrawer: React.FC<ChatDrawerProps> = ({
  side,
  role,
  active,
  title,
  onClose,
  children,
}) => {
  const classNames = clsx({
    "chat__drawer": true,
    [`chat__drawer--${side}`]: true,
    [`chat__drawer--${role}`]: true,
    "chat__drawer--active": active,
  });

  const Icon = side === "left" ? ImCircleLeft : ImCircleRight;

  return (
    <div className={classNames}>
      <div className="chat__drawer__header">
        <span>{title}</span>
        <Icon className="chat__drawer__header__close_button" onClick={onClose} />
      </div>
      {active && <ChatDrawerBody onClose={onClose}>{children}</ChatDrawerBody>}
    </div>
  );
};

type ChatDrawerButtonProps = {
  icon: IconType;
  onClick: () => void;
  active: boolean;
};

const ChatDrawerButton: React.FC<ChatDrawerButtonProps> = ({ icon: Icon, onClick, active }) => {
  const className = clsx({
    "chat__nav__icon": true,
    "chat__nav__icon--active": active,
  });

  const handleClick = (e: React.MouseEvent) => {
    if (!active) {
      e.stopPropagation();
      onClick();
    }
  };

  return <Icon className={className} onClickCapture={handleClick} />;
};

const TestContent: React.FC = () => (
  <Scrollbars autoHide={true}>
    <div style={{ height: "1000px" }} />
  </Scrollbars>
);

const ChatThing: React.FC = () => {
  const [state, { sendMessage }] = useChat();
  const [activePanel, setActivePanel] = useState(ChatDrawerRole.Settings);

  const closePanel = useCallback(() => setActivePanel(ChatDrawerRole.None), []);

  return (
    <>
      <div className="chat__messages">
        <ChatDrawer
          title="Emotes"
          side="left"
          role={ChatDrawerRole.Emotes}
          active={activePanel === ChatDrawerRole.Emotes}
          onClose={closePanel}
        >
          <TestEmotes />
        </ChatDrawer>
        <ChatDrawer
          title="Whispers"
          side="left"
          role={ChatDrawerRole.Whispers}
          active={activePanel === ChatDrawerRole.Whispers}
          onClose={closePanel}
        >
          <TestContent />
        </ChatDrawer>
        <ChatDrawer
          title="Settings"
          side="right"
          role={ChatDrawerRole.Settings}
          active={activePanel === ChatDrawerRole.Settings}
          onClose={closePanel}
        >
          <TestSettings />
        </ChatDrawer>
        <ChatDrawer
          title="Users"
          side="right"
          role={ChatDrawerRole.Users}
          active={activePanel === ChatDrawerRole.Users}
          onClose={closePanel}
        >
          <TestContent />
        </ChatDrawer>
        <Scroller
          renderMessage={({ index, style }: MessageProps) => (
            <Message message={state.messages[index]} style={style} />
          )}
          messageCount={state.messages.length}
        />
      </div>
      <div className="chat__footer">
        <Composer
          emotes={state.emotes}
          modifiers={state.modifiers}
          tags={state.tags}
          nicks={state.nicks}
          onMessage={sendMessage}
        />
      </div>
      <div className="chat__nav">
        <div className="chat__nav__left">
          <ChatDrawerButton
            icon={BiSmile}
            active={activePanel === ChatDrawerRole.Emotes}
            onClick={() => setActivePanel(ChatDrawerRole.Emotes)}
          />
          <ChatDrawerButton
            icon={BiConversation}
            active={activePanel === ChatDrawerRole.Whispers}
            onClick={() => setActivePanel(ChatDrawerRole.Whispers)}
          />
        </div>
        <div className="chat__nav__right">
          <ChatDrawerButton
            icon={FiSettings}
            active={activePanel === ChatDrawerRole.Settings}
            onClick={() => setActivePanel(ChatDrawerRole.Settings)}
          />
          <ChatDrawerButton
            icon={HiOutlineUserGroup}
            active={activePanel === ChatDrawerRole.Users}
            onClick={() => setActivePanel(ChatDrawerRole.Users)}
          />
        </div>
      </div>
    </>
  );
};

const ChatPanel: React.FC = () => {
  const [closed, toggleClosed] = useToggle(false);

  const className = clsx({
    "home_page__right": true,
    "home_page__right--closed": closed,
  });

  return (
    <aside className={className}>
      <button className="home_page__right__toggle_on" onClick={toggleClosed}>
        <BsArrowBarLeft size={22} />
      </button>
      <div className="home_page__right__body">
        <header className="home_page__subheader">
          <button className="home_page__right__toggle_off" onClick={toggleClosed}>
            <BsArrowBarRight size={22} />
          </button>
        </header>
        <header className="home_page__chat__promo"></header>
        <div className="home_page__chat chat">
          <Provider
            networkKey={Base64.toUint8Array("ewOeQgqCCXYwVmR+nZIcbLfDszuIgV8l0Xj0OVa5Vw4=")}
            serverKey={Base64.toUint8Array("fHyr7+njRTRAShsdcDB1vOz9373dtPA476Phw+DYh0Q=")}
          >
            <ChatThing />
          </Provider>
        </div>
      </div>
    </aside>
  );
};

export default ChatPanel;
