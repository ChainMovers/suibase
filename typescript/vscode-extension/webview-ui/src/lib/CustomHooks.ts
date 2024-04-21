import { useContext } from "react";
import { MessageContext } from "./MessageContext"; // Adjust the import path as necessary

export const useMessage = () => useContext(MessageContext);
