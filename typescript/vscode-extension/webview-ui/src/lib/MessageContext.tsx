import React, { createContext, useState, ReactNode } from "react";

export const MessageContext = createContext({
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    message: null as any,
    // eslint-disable-next-line @typescript-eslint/no-explicit-any, @typescript-eslint/ban-types, @typescript-eslint/no-unused-vars
    setMessage: (_message: any) => {},
});

interface MessageProviderProps {
        children: ReactNode;
}

export const MessageProvider: React.FC<MessageProviderProps> = ({ children }) => {
        const [message, setMessage] = useState<string | null>(null);

        return <MessageContext.Provider value={{ message, setMessage }}>{children}</MessageContext.Provider>;
};
