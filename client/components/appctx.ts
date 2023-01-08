import { createContext } from 'react'
import { IMessage } from './messages'

export type AppContextType = {
  messages: IMessage[];
  setMessages: (messages: IMessage[]) => void;
  loggedIn: boolean;
  setLoggedIn: (value: boolean) => void;
  showNewMessage: boolean;
  setShowNewMessage: (value: boolean) => void;
  shouldRefresh: boolean;
  toggleRefresh: () => void;
  myEmail: string;
  setMyEmail: (value: string) => void;
  showOwner: boolean;
  setShowOwner: (value: boolean) => void;
  showLoginPrompt: boolean;
  setShowLoginPrompt: (value: boolean) => void;
  loading: boolean;
  setLoading: (value: boolean) => void;
};

export const AppContext = createContext<AppContextType | null>(null)
