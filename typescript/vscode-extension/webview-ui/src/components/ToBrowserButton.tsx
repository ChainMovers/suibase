import { Link } from "@mui/material";

import OpenInNewIcon from "@mui/icons-material/OpenInNew";

interface ToBrowserButtonProps {
  network: string; // e.g. "localnet", "testnet"...
  type: string; // One of "package", "txn", "object"
  id: string; // ID to put in the URL.
}

const ToBrowserButton = ({ network, type, id }: ToBrowserButtonProps) => {
  // If network is "localnet", then use localhost.
  // TODO Use config from backend.
  let url: string;
  if (network === "localnet") {
    if (type === "txn") {
      url = `http://localhost:44380/txblock/${id}`;
    } else {
      url = `http://localhost:44380/object/${id}`;
    }
  } else {
    // Use suiscan.xyz for other networks.
    if (type === "package") {
      url = `https://suiscan.xyz/${network}/object/${id}/contracts`;
    } else if (type === "txn") {
      url = `https://suiscan.xyz/${network}/tx/${id}`;
    } else {
      url = `https://suiscan.xyz/${network}/object/${id}`;
    }
  }

  return (
    <Link
      className="icon"
      color="inherit"
      href={url}
      target="_blank"
      rel="noopener noreferrer"
      sx={{ width: "20px", display: "flex", alignItems: "center", justifyContent: "center" }}>
      <OpenInNewIcon sx={{ height: "16px" }} />
    </Link>
  );
};

export default ToBrowserButton;
