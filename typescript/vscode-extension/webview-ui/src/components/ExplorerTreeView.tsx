/* eslint-disable @typescript-eslint/no-unused-vars */
import "./styles/ExplorerTreeView.css";
import { TreeViewBaseItem } from "@mui/x-tree-view";
import MuiTreeView from "./MuiTreeView";
import { TREE_ID_INSERT_ADDR, TREE_ITEM_ALL_PACKAGES, TREE_ITEM_ID_RECENT_PACKAGES_EMPTY, TREE_ITEM_PACKAGE, TREE_ITEM_RECENT_PACKAGES } from "../common/Consts";
import { SuibaseJsonWorkdirPackages } from "../common/SuibaseJson";
import { useEffect, useState } from "react";



  // The first character of 'id' is a "type" that assists UI rendering.
  // See Consts.ts for more about TREE_ITEM_x  
  const empty_tree: TreeViewBaseItem[] = [
  {
    id: TREE_ITEM_RECENT_PACKAGES,
    label: 'Recent Packages',
    children:[
      { id: TREE_ITEM_ID_RECENT_PACKAGES_EMPTY, label: '' },
    ]
  }
  ];

export class ViewExplorerState {
  // This class is for data "storage" only.
  //
  // It is "deep cloned" with a method that may change 
  // in the future. Consequently, do not use:
  //   - getter/setter 
  //   - any functions  
  //
  // reference: https://stackoverflow.com/questions/28150967/typescript-cloning-object
  workdir: string;  
  workdirIdx: number;
  methodUuid: string; // Versioning for the backend method used to build the tree.
  dataUuid: string;   // Versioning for the data returned by this backend method.
  tree: TreeViewBaseItem[];

  constructor(workdir: string, workdirIdx: number) {
    this.workdir = workdir;
    this.workdirIdx = workdirIdx;
    this.methodUuid = "";
    this.dataUuid = "";
    this.tree = empty_tree;
  }
}

//import CustomContentTreeView from "./MuiTreeView";
//import { Typography } from "@mui/material";

/*
function json_to_tree_recursive(obj: any): any {  
  const tree = new TreeObject("");
  for (const key in obj) {
    console.log(`key: ${key} typeof: ${typeof obj[key]} obj[key]: ${obj[key]}`);
    if (typeof obj[key] === "object") {
      tree.children.push({
        name: key,
        children: json_to_tree_recursive(obj[key]),
      });
    } else if (typeof obj[key] === "string") {
      tree.children.push(new TreeObject(`${key}: ${obj[key]}`));
    }
  }
  return tree;
}


function jsonStringToTree(json: string): any {
  //const obj = JSON.parse(json);
  const tree = json_to_tree_recursive(json);
  return tree;
}


function jsonObjToTree(json: any): any {
  const tree = json_to_tree_recursive(json);
  return tree;
}*/


function updateViewExplorerState(state: ViewExplorerState, workdir_json: SuibaseJsonWorkdirPackages): ViewExplorerState {
  // Returns a new instance of ViewExplorerState if there is any changes. Otherwise, it returns the "state" parameter.  
  // let newState = structuredClone(state); <-- How to deep clone if ever needed.

  // Returns an empty tree if no backend data
  if (!workdir_json.isLoaded) {    
    if (state.tree == empty_tree) {
      return state; // No changes. Current state already empty.
    }    
    // Clear by returning a new state with empty tree.
    return new ViewExplorerState(state.workdir, state.workdirIdx);
  }
  
  // Fast-detection of backend changes
  if (state.methodUuid === workdir_json.getMethodUuid() && state.dataUuid === workdir_json.getDataUuid()) {
    return state; // No changes. Return current up-to-date state.
  }

  // Changes were detected, so build a new ViewExplorerState.
  // TODO Optimization needed for very large trees (not rebuild from scratch!?)
  let newState = new ViewExplorerState(state.workdir, state.workdirIdx);
  newState.methodUuid = workdir_json.getMethodUuid();
  newState.dataUuid = workdir_json.getDataUuid();
  newState.tree = [];

  // Example of json with one package in the moveConfigs map (the key is the Suibase Module UUID):
  //  {"header":{"method":"getWorkdirPackages","methodUuid":"...","dataUuid":"...","key":"localnet"},
  //   "moveConfigs":{ "...UUID...":
  //                    {
  //                      "path":"home/user/rust/demo-app/move",
  //                      "latestPackage":{"packageId":"f774c6336e5e0cbb6175dd14f1cc96c4bdde15639e6e1fd876de0b54ef97de88","packageName":"demo","packageTimestamp":"1715418730452","initObjects":null},
  //                      "olderPackages":[{"packageId":"67c90dd5fa3a9ce5073576ece3a66d77fd5d14f0526f6c39e6acc7589682c3c1","packageName":"demo","packageTimestamp":"1715385760103","initObjects":null}],
  //                      "trackingState":2
  //                    }
  //                 }
  //  }
  let json = workdir_json.getJson();
    
  if (!json) {
    console.log( "Missing json in workdir_json: " + JSON.stringify(workdir_json));
    return state;
  }

  if (!json.moveConfigs) {
    console.log( "Missing moveConfigs in workdir_json: " + JSON.stringify(workdir_json));
    return state;
  }  
  const moveConfigs = json.moveConfigs;

  // Create the top level tree folders.
  let recent_packages_folder: TreeViewBaseItem[] = [];
  let all_packages_folder: TreeViewBaseItem[] = [];
  
  for (let uuid in moveConfigs ) {
    const config = moveConfigs[uuid];
    if (config.latestPackage != null) {
      let folderPath = `${TREE_ITEM_RECENT_PACKAGES}-${uuid}`;
      let packageItem = buildTreeViewPackage(folderPath, config.latestPackage);
      if (packageItem) recent_packages_folder.push(packageItem);

      folderPath = `${TREE_ITEM_ALL_PACKAGES}-${uuid}`;
      packageItem = buildTreeViewPackage(folderPath, config.latestPackage);
      if (packageItem) all_packages_folder.push(packageItem);   
    }
    if (config.olderPackages != null) {
      for (let olderPackage of config.olderPackages) {
        const id = `${TREE_ITEM_ALL_PACKAGES}-${uuid}`;
        const packageItem = buildTreeViewPackage(id, olderPackage);
        if (packageItem) all_packages_folder.push(packageItem);        
      }
    }
  }

  // Special case of no recent packages (we want a "special" item to display UI help).
  if (recent_packages_folder.length == 0) {
    recent_packages_folder = [{ id: TREE_ITEM_ID_RECENT_PACKAGES_EMPTY, label: '' }];
  }

  // Add top level to the tree.  
  newState.tree.push({ id: TREE_ITEM_RECENT_PACKAGES, label: 'Recent Packages', children: recent_packages_folder });
  newState.tree.push({ id: TREE_ITEM_ALL_PACKAGES, label: 'All Packages', children: all_packages_folder });
  
  return newState;
}

function buildTreeViewPackage(folderPath: string, json: any ): TreeViewBaseItem | undefined {
  // json example: {"packageId":"f774c6336e5e0cbb6175dd14f1cc96c4bdde15639e6e1fd876de0b54ef97de88",
  //                "packageName":"demo",
  //                "packageTimestamp":"1715418730452",
  //                "initObjects":null
  //               }
  const packageName = json.packageName;
  if (!packageName) {
    console.log( "Missing packageName in package json: " + JSON.stringify(json));
    return undefined;
  }
  const packageId = json.packageId;
  if (!packageId) {
    console.log( "Missing packageId in package json: " + JSON.stringify(json));
    return undefined;
  }
  
  // [TREE_ID_INSERT_ADDR] is a special string that will enhance the label 
  // with address shortening, tooltip, clipboard copy etc... of the packageId field.
  const label = `${TREE_ID_INSERT_ADDR}::${packageName}`;
  const id = `${TREE_ITEM_PACKAGE}-${folderPath}-${packageId}`;

  // TODO Add init objects as children.

  return { id: id, label: label };
}

// Define the props type
interface ExplorerTreeViewProps {  
  workdir: string;
  workdirIdx: number;
  packagesTrigger: boolean; 
  packagesJson: SuibaseJsonWorkdirPackages;  
}

export function ExplorerTreeView({ workdir, workdirIdx, packagesTrigger, packagesJson }: ExplorerTreeViewProps) {
  const [explorerState, setExplorerState] = useState<ViewExplorerState>(new ViewExplorerState(workdir, workdirIdx));
  
  useEffect(() => {
    // Update the ViewExplorerState (suitable for display) using the SuibaseJsonWorkdirPackages.
    //
    // For better UX with the underlying MUI TreeView object, the function 
    // updateViewExplorerState preserves tree view "id" whenever possible.
    //
    // newState is a deep clone of explorerState + changes observed in packages_json.
    //
    // It is necessary to create a new instance of ViewExplorerState for setExplorerState to 
    // trigger a re-render.
    try {
      const newState = updateViewExplorerState(explorerState, packagesJson);
      if( newState !== explorerState ) {
        setExplorerState(newState);
      }
    } catch (error) {
      console.error(`Error updating ExplorerTreeView: ${error}`);
    }
  }, [workdir,workdirIdx,packagesTrigger,packagesJson]);

  // <pre>{JSON.stringify(explorerState.tree,null,2)}</pre>
  return (
    <>      
      <MuiTreeView items={explorerState.tree} workdir={workdir}/>
    </>);  
}
