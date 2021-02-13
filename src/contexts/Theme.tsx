import clsx from "clsx";
import React from "react";

type ColorScheme = "dark" | "light";

interface State {
  colorScheme: ColorScheme;
  navOrder: number[];
}

type Action =
  | {
      type: "SET_COLOR_SCHEME";
      colorScheme: ColorScheme;
    }
  | {
      type: "SET_NAV_ORDER";
      navOrder: number[];
    };

const initialState: State = {
  colorScheme: "dark",
  navOrder: [],
};

const ProfileContext = React.createContext<[State, (action: Action) => void]>(null);

const themeReducer = (state: State, action: Action): State => {
  switch (action.type) {
    case "SET_COLOR_SCHEME":
      return {
        ...state,
        colorScheme: action.colorScheme,
      };
    case "SET_NAV_ORDER":
      console.log(action.navOrder);
      return {
        ...state,
        navOrder: action.navOrder,
      };
    default:
      return state;
  }
};

export const useTheme = (): [State, typeof actions] => {
  const [state, dispatch] = React.useContext(ProfileContext);
  const setColorScheme = (colorScheme: ColorScheme) =>
    dispatch({
      type: "SET_COLOR_SCHEME",
      colorScheme,
    });

  const setNavOrder = (navOrder: number[]) =>
    dispatch({
      type: "SET_NAV_ORDER",
      navOrder,
    });

  const actions = {
    setColorScheme,
    setNavOrder,
  };
  return [state, actions];
};

export const Provider: React.FC = ({ children }) => {
  const [state, dispatch] = React.useReducer(themeReducer, initialState);

  return (
    <ProfileContext.Provider value={[state, dispatch]}>
      <div className={clsx("app", `app--${state.colorScheme}`)}>{children}</div>
    </ProfileContext.Provider>
  );
};

Provider.displayName = "Profile.Provider";
