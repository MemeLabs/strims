import clsx from "clsx";
import * as React from "react";

type ColorScheme = "dark" | "light";

interface State {
  colorScheme: ColorScheme;
}

type Action =
  | {
      type: "SET_COLOR_SCHEME";
      colorScheme: ColorScheme;
    }
  | {
      type: "MEME";
    };

const initialState: State = {
  colorScheme: "dark",
};

const ProfileContext = React.createContext<[State, (action: Action) => void]>(null);

const themeReducer = (state: State, action: Action): State => {
  switch (action.type) {
    case "SET_COLOR_SCHEME":
      return {
        ...state,
        colorScheme: action.colorScheme,
      };
    default:
      return state;
  }
};

export const useTheme = () => {
  const [state, dispatch] = React.useContext(ProfileContext);
  const setColorScheme = (colorScheme: ColorScheme) =>
    dispatch({
      type: "SET_COLOR_SCHEME",
      colorScheme,
    });

  const actions = {
    setColorScheme,
  };
  return [state, actions] as [State, typeof actions];
};

export const Provider = ({ children }: any) => {
  const [state, dispatch] = React.useReducer(themeReducer, initialState);

  return (
    <ProfileContext.Provider value={[state, dispatch]}>
      <div className={clsx("app", `app--${state.colorScheme}`)}>{children}</div>
    </ProfileContext.Provider>
  );
};

Provider.displayName = "Profile.Provider";
