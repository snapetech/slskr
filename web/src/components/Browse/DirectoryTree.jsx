import React, { useCallback, useEffect, useState } from 'react';
import { Button, Checkbox, Icon, List } from 'semantic-ui-react';

const DirectoryNode = ({
  directory,
  expandedPaths,
  level,
  onDownloadSelected,
  onToggleExpand,
  onToggleSelect,
  selectedDirectoryName,
  selectedPaths,
}) => {
  const directoryName =
    typeof directory?.name === 'string' ? directory.name : 'Unknown folder';
  const isExpanded = expandedPaths.has(directoryName);
  const isSelected = selectedPaths.has(directoryName);
  const isActive = directoryName === selectedDirectoryName;
  const children = Array.isArray(directory?.children)
    ? directory.children
    : [];
  const hasChildren = children.length > 0;
  const folderName = directoryName.split('\\').pop().split('/').pop();

  return (
    <List.Item style={{ paddingLeft: level > 0 ? '1em' : 0 }}>
      <List.Content>
        <div style={{ alignItems: 'center', display: 'flex', gap: '4px' }}>
          {/* Expand/Collapse toggle */}
          {hasChildren ? (
            <Icon
              name={isExpanded ? 'caret down' : 'caret right'}
              onClick={() => onToggleExpand(directoryName)}
              style={{ cursor: 'pointer', width: '16px' }}
            />
          ) : (
            <span style={{ width: '16px' }} />
          )}

          {/* Checkbox */}
          <Checkbox
            checked={isSelected}
            onChange={() => onToggleSelect(directory)}
            style={{ marginRight: '4px' }}
          />

          {/* Folder icon */}
          <Icon
            className={directory.locked ? 'locked' : ''}
            name={
              directory.locked ? 'lock' : isExpanded ? 'folder open' : 'folder'
            }
            style={{ opacity: directory.locked ? 0.5 : 1 }}
          />

          {/* Folder name */}
          <span
            onClick={() => onToggleExpand(directory.name)}
            onKeyDown={(event) => {
              if (event.key === 'Enter' || event.key === ' ') {
                onToggleExpand(directoryName);
              }
            }}
            role="button"
            style={{
              color: isActive ? '#2185d0' : 'inherit',
              cursor: 'pointer',
              fontWeight: isActive ? 'bold' : 'normal',
              opacity: directory.locked ? 0.5 : 1,
            }}
            tabIndex={0}
          >
            {folderName}
          </span>

          {/* File count badge */}
          {directory.fileCount > 0 && (
            <span
              style={{
                background: '#555',
                borderRadius: '10px',
                color: '#fff',
                fontSize: '0.75em',
                marginLeft: '6px',
                padding: '1px 6px',
              }}
            >
              {directory.fileCount}
            </span>
          )}
        </div>

        {/* Children - only show when expanded */}
        {hasChildren && isExpanded && (
          <List.List style={{ marginTop: '4px' }}>
            {children.map((child) => (
              <DirectoryNode
                directory={child}
                expandedPaths={expandedPaths}
                key={child.name}
                level={level + 1}
                onDownloadSelected={onDownloadSelected}
                onToggleExpand={onToggleExpand}
                onToggleSelect={onToggleSelect}
                selectedDirectoryName={selectedDirectoryName}
                selectedPaths={selectedPaths}
              />
            ))}
          </List.List>
        )}
      </List.Content>
    </List.Item>
  );
};

const DirectoryTree = ({
  downloadPending = false,
  onDownload,
  onSelect,
  selectedDirectoryName,
  tree = [],
}) => {
  const [expandedPaths, setExpandedPaths] = useState(new Set());
  const [selectedPaths, setSelectedPaths] = useState(new Set());
  const safeTree = Array.isArray(tree)
    ? tree.filter(
        (directory) =>
          directory &&
          typeof directory === 'object' &&
          typeof directory.name === 'string',
      )
    : [];

  useEffect(() => {
    const names = new Set();
    const collectNames = (nodes) => {
      nodes.forEach((node) => {
        names.add(node.name);
        if (Array.isArray(node.children)) collectNames(node.children);
      });
    };
    collectNames(safeTree);
    setSelectedPaths((previous) => {
      const next = new Set(
        Array.from(previous).filter((path) => names.has(path)),
      );
      return next.size === previous.size ? previous : next;
    });
    setExpandedPaths((previous) => {
      const next = new Set(
        Array.from(previous).filter((path) => names.has(path)),
      );
      return next.size === previous.size ? previous : next;
    });
  }, [safeTree]);

  const toggleExpand = useCallback((path) => {
    setExpandedPaths((previous) => {
      const updated = new Set(previous);

      if (updated.has(path)) {
        updated.delete(path);
      } else {
        updated.add(path);
      }

      return updated;
    });
  }, []);

  // Get all descendant paths of a directory
  const getDescendantPaths = useCallback((directory) => {
    if (!directory || typeof directory.name !== 'string') return [];
    const paths = [directory.name];

    if (Array.isArray(directory.children)) {
      for (const child of directory.children) {
        paths.push(...getDescendantPaths(child));
      }
    }

    return paths;
  }, []);

  const toggleSelect = useCallback(
    (directory) => {
      setSelectedPaths((previous) => {
        const updated = new Set(previous);
        const descendantPaths = getDescendantPaths(directory);
        const isCurrentlySelected = updated.has(directory.name);

        if (isCurrentlySelected) {
          // Deselect this directory and all descendants
          for (const path of descendantPaths) {
            updated.delete(path);
          }
        } else {
          // Select this directory and all descendants
          for (const path of descendantPaths) {
            updated.add(path);
          }
        }

        return updated;
      });

      // Also call onSelect to show files in the detail view
      onSelect(null, directory);
    },
    [getDescendantPaths, onSelect],
  );

  const handleDownloadSelected = useCallback(() => {
    // Find all selected directories from the tree (top-level only to avoid duplicates)
    const findSelectedDirectories = (nodes) => {
      const found = [];

      for (const node of nodes) {
        if (selectedPaths.has(node.name)) {
          // Include this node with its full subtree
          found.push(node);
        } else if (node.children) {
          // Only recurse if parent isn't selected (to avoid duplicates)
          found.push(...findSelectedDirectories(node.children));
        }
      }

      return found;
    };

    const selectedDirectories = findSelectedDirectories(safeTree);

    if (selectedDirectories.length === 0) {
      return;
    }

    // Create a combined directory that preserves the full tree structure
    // by including selected directories as children so recursive file collection works
    const combinedDirectory = {
      children: selectedDirectories,
      files: [],
      name: `${selectedDirectories.length} selected folders`,
    };

    onDownload(combinedDirectory);
  }, [onDownload, safeTree, selectedPaths]);

  const expandAll = useCallback(() => {
    const getAllPaths = (nodes) => {
      const paths = [];

      for (const node of nodes) {
        if (node.children && node.children.length > 0) {
          paths.push(node.name);
          paths.push(...getAllPaths(node.children));
        }
      }

      return paths;
    };

    setExpandedPaths(new Set(getAllPaths(safeTree)));
  }, [safeTree]);

  const collapseAll = useCallback(() => {
    setExpandedPaths(new Set());
  }, []);

  const clearSelection = useCallback(() => {
    setSelectedPaths(new Set());
  }, []);

  const selectedCount = selectedPaths.size;

  return (
    <div>
      {/* Toolbar */}
      <div
        style={{
          alignItems: 'center',
          borderBottom: '1px solid #333',
          display: 'flex',
          gap: '8px',
          marginBottom: '8px',
          paddingBottom: '8px',
        }}
      >
        <Button
          compact
          onClick={expandAll}
          size="tiny"
        >
          <Icon name="expand" /> Expand All
        </Button>
        <Button
          compact
          onClick={collapseAll}
          size="tiny"
        >
          <Icon name="compress" /> Collapse All
        </Button>
        {selectedCount > 0 && (
          <>
            <Button
              compact
              onClick={clearSelection}
              size="tiny"
            >
              <Icon name="x" /> Clear ({selectedCount})
            </Button>
            <Button
              color="green"
              compact
              disabled={downloadPending}
              loading={downloadPending}
              onClick={handleDownloadSelected}
              size="tiny"
            >
              <Icon name="download" /> Download Selected
            </Button>
          </>
        )}
      </div>

      {/* Tree */}
      <List className="browse-folderlist-list">
        {safeTree.map((directory) => (
          <DirectoryNode
            directory={directory}
            expandedPaths={expandedPaths}
            key={directory.name}
            level={0}
            onDownloadSelected={handleDownloadSelected}
            onToggleExpand={toggleExpand}
            onToggleSelect={toggleSelect}
            selectedDirectoryName={selectedDirectoryName}
            selectedPaths={selectedPaths}
          />
        ))}
      </List>
    </div>
  );
};

export default DirectoryTree;
