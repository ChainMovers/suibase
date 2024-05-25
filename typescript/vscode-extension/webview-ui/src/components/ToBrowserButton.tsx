import { Link} from '@mui/material';

import OpenInNewIcon from '@mui/icons-material/OpenInNew';

interface ToBrowserButtonProps {
    network: string; // e.g. "testnet"
    type: string; // e.g. "package"
    id: string; // e.g. a package ID in hexadecimal.    
}

const ToBrowserButton = ({network,type,id}: ToBrowserButtonProps) => {
    const url = `https://${network}.suivision.xyz/${type}/${id}`;

    return (        
        <Link className="icon" color='inherit' href={url} target="_blank" rel="noopener noreferrer" sx={{width: '20px', display: 'flex', alignItems: 'center', justifyContent: 'center'}}>            
          <OpenInNewIcon sx={{height: '16px'}}/>            
        </Link>        
    );
}

export default ToBrowserButton