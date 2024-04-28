/* eslint-disable @typescript-eslint/no-unused-vars */
//import TreeView, { flattenTree } from "react-accessible-treeview";
//import "./styles/DebugTreeView.css";
import { Typography } from "@mui/material";

// A function that convert any JSON string into an object as follows:
//
// const object = {
//    name: "root",
//    children: [
//      { name: "example_number: 10"},
//      { name: "example_string: 'hello'"},
//      { name: "example_boolean: true"},
//      { name: "example_object {}" children [{name: "example_number: 10"}, {name: "example_string: 'hello'"}...]},
//    ]
//}
//
// Object can be nested into object as { name: "", children: []}
/*
class TreeObject {
  name: string;
  children: TreeObject[];
  constructor(name: string) {
    this.name = name;
    this.children = [];
  }
}*/

// eslint-disable-next-line @typescript-eslint/no-explicit-any
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

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function jsonStringToTree(json: string): any {
  //const obj = JSON.parse(json);
  const tree = json_to_tree_recursive(json);
  return tree;
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function jsonObjToTree(json: any): any {
  const tree = json_to_tree_recursive(json);
  return tree;
}*/

/*
const folder = {
  name: "",
  children: [
    {
      name: "src",
      children: [{ name: "index.js" }, { name: "styles.css" }],
    },
    {
      name: "node_modules",
      children: [
        {
          name: "react-accessible-treeview",
          children: [{ name: "index.js" }],
        },
        { name: "react", children: [{ name: "index.js" }] },
      ],
    },
    {
      name: ".npmignore",
    },
    {
      name: "package.json",
    },
    {
      name: "webpack.config.js",
    },
  ],
};*/

// Define the props type
interface DebugTreeViewObjProps {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  jsonObj: any;  
}

export function DebugTreeViewObj({ jsonObj }: DebugTreeViewObjProps) {
  // TODO Get this to work...
  const jsonStr = JSON.stringify(jsonObj);
  return (
    <div>
      <Typography variant="body2">{jsonStr}</Typography>
    </div>
  );

  /*
  const jsonStr = JSON.stringify(jsonObj);  
  const data = flattenTree(jsonStringToTree(jsonStr));
  return (
    <div>
      <Typography variant="body2">{jsonStr}</Typography>
      <Typography variant="body2">{JSON.stringify(data)}</Typography>
      <div className="debugtree">
        <TreeView
          data={data}
          aria-label="debugtree tree"
          nodeRenderer={({
            element,
            isBranch,
            isExpanded,
            getNodeProps,
            level,
          }) => (
            <div {...getNodeProps()} style={{ paddingLeft: 20 * (level - 1) }}>
              {isBranch ? (
                <FolderIcon isOpen={isExpanded} />
              ) : (
                <FileIcon filename={element.name} />
              )}

              {element.name}
            </div>
          )}
        />
      </div>
    </div>
  );*/
}

interface DebugTreeViewStrProps {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  jsonStr: string;  
}

export function DebugTreeViewStr({ jsonStr }: DebugTreeViewStrProps) {
  // TODO Get this to work...
  return (
    <div>
      <Typography variant="body2">{jsonStr}</Typography>
    </div>
  );

  /*
  const data = flattenTree(jsonStringToTree(jsonStr));
  return (
    <div>
      <div className="debugtree">
        <TreeView
          data={data}
          aria-label="debugtree tree"
          nodeRenderer={({
            element,
            isBranch,
            isExpanded,
            getNodeProps,
            level,
          }) => (
            <div {...getNodeProps()} style={{ paddingLeft: 20 * (level - 1) }}>
              {isBranch ? (
                <FolderIcon isOpen={isExpanded} />
              ) : (
                <FileIcon filename={element.name} />
              )}

              {element.name}
            </div>
          )}
        />
      </div>
    </div>
  );
  */
}

/*
const FolderIcon = ({ isOpen }: { isOpen: boolean }) =>
  isOpen ? (
    <i className="icon codicon codicon-folder-opened"></i>
  ) : (
    <i className="icon codicon codicon-folder"></i>
  );

const FileIcon = ({ filename }: { filename: string }) => {
  const extension = filename.slice(filename.lastIndexOf(".") + 1);
  switch (extension) {
    case "js":
      return <div className="icon"><i className="codicon codicon-json"></i></div>;
    case "css":
      return <div className="icon"><i className="codicon codicon-json"></i></div>;
    case "json":
      return <div className="icon"><i className="codicon codicon-json"></i></div>;
    case "npmignore":
      return <div className="icon"><i className="codicon codicon-json"></i></div>;
    default:
      return null;
  }
};
*/