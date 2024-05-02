import React, { createContext, useState, ReactNode } from "react";

export const MessageContext = createContext({
    
    message: null as any,
    setMessage: (_message: any) => {},
});

interface MessageProviderProps {
        children: ReactNode;
}

export const MessageProvider: React.FC<MessageProviderProps> = ({ children }) => {
        const [message, setMessage] = useState<string | null>(null);

        return <MessageContext.Provider value={{ message, setMessage }}>{children}</MessageContext.Provider>;
};
