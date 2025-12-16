import { useCallback, useRef, useEffect, useMemo } from 'react';
import ForceGraph2D, { ForceGraphMethods } from 'react-force-graph-2d';
import type { GraphNode, GraphLink } from '@/types';

interface PeerGraphProps {
  nodes: GraphNode[];
  links: GraphLink[];
  onNodeClick?: (nodeId: string) => void;
  selectedNodeId?: string | null;
}

// Univrs.io bioluminescent color palette
const COLORS = {
  void: '#0a0d0b',
  deepEarth: '#0f1411',
  forestFloor: '#141a16',
  borderSubtle: '#2a3a30',
  glowCyan: '#00ffd5',
  glowCyanDim: 'rgba(0, 255, 213, 0.25)',
  glowGold: '#ffd700',
  sporePurple: '#b088f9',
  myceliumWhite: '#e8f4ec',
  softGray: '#8a9a8f',
};

export function PeerGraph({ nodes, links, onNodeClick, selectedNodeId }: PeerGraphProps) {
  const graphRef = useRef<ForceGraphMethods>();

  // Memoize graph data to prevent re-renders from creating new objects
  const graphDataMemo = useMemo(() => ({ nodes, links }), [nodes, links]);

  // Color scale based on reputation - univrs.io palette
  const getNodeColor = useCallback((node: GraphNode) => {
    if (node.isLocal) return COLORS.sporePurple; // purple for local node
    const score = node.reputation;
    if (score >= 0.9) return COLORS.glowCyan; // cyan - excellent
    if (score >= 0.7) return COLORS.glowGold; // gold - good
    if (score >= 0.5) return COLORS.softGray; // gray - neutral
    if (score >= 0.3) return '#f59e0b'; // amber - poor
    return '#ef4444'; // red - untrusted
  }, []);

  const getNodeSize = useCallback((node: GraphNode) => {
    return node.isLocal ? 12 : 8 + node.reputation * 4;
  }, []);

  // Highlight selected node with bioluminescent glow
  const nodeCanvasObject = useCallback(
    (node: any, ctx: CanvasRenderingContext2D, globalScale: number) => {
      if (node.x === undefined || node.y === undefined) return;
      const graphNode = node as GraphNode;

      const label = graphNode.name || String(graphNode.id).slice(0, 8);
      const fontSize = 12 / globalScale;
      const size = getNodeSize(graphNode);
      const color = getNodeColor(graphNode);
      const isSelected = graphNode.id === selectedNodeId;

      // Draw outer glow for all nodes (bioluminescent effect)
      ctx.beginPath();
      ctx.arc(node.x, node.y, size + 6, 0, 2 * Math.PI);
      const gradient = ctx.createRadialGradient(node.x, node.y, size, node.x, node.y, size + 6);
      gradient.addColorStop(0, `${color}40`);
      gradient.addColorStop(1, 'transparent');
      ctx.fillStyle = gradient;
      ctx.fill();

      // Draw stronger glow for selected node
      if (isSelected) {
        ctx.beginPath();
        ctx.arc(node.x, node.y, size + 10, 0, 2 * Math.PI);
        const selectGradient = ctx.createRadialGradient(node.x, node.y, size, node.x, node.y, size + 10);
        selectGradient.addColorStop(0, `${color}60`);
        selectGradient.addColorStop(1, 'transparent');
        ctx.fillStyle = selectGradient;
        ctx.fill();
      }

      // Draw node
      ctx.beginPath();
      ctx.arc(node.x, node.y, size, 0, 2 * Math.PI);
      ctx.fillStyle = color;
      ctx.fill();

      // Draw border
      ctx.strokeStyle = isSelected ? COLORS.myceliumWhite : COLORS.borderSubtle;
      ctx.lineWidth = isSelected ? 2 / globalScale : 1 / globalScale;
      ctx.stroke();

      // Draw label with Syne font style
      ctx.font = `600 ${fontSize}px Syne, sans-serif`;
      ctx.textAlign = 'center';
      ctx.textBaseline = 'top';
      ctx.fillStyle = COLORS.myceliumWhite;
      ctx.fillText(label, node.x, node.y + size + 4);
    },
    [getNodeColor, getNodeSize, selectedNodeId]
  );

  // Safe click handler that extracts just the ID
  const handleNodeClick = useCallback((node: any) => {
    if (node && node.id && onNodeClick) {
      onNodeClick(node.id);
    }
  }, [onNodeClick]);

  useEffect(() => {
    // Fit graph to view on initial load
    if (graphRef.current && nodes.length > 0) {
      setTimeout(() => {
        graphRef.current?.zoomToFit(400, 50);
      }, 500);
    }
  }, [nodes.length]);

  return (
    <div className="w-full h-full bg-forest-floor border border-border-subtle rounded-lg overflow-hidden">
      <ForceGraph2D
        ref={graphRef}
        graphData={graphDataMemo}
        nodeCanvasObject={nodeCanvasObject}
        nodePointerAreaPaint={(node, color, ctx) => {
          const graphNode = node as GraphNode & { x?: number; y?: number };
          if (graphNode.x === undefined || graphNode.y === undefined) return;
          const size = getNodeSize(graphNode);
          ctx.beginPath();
          ctx.arc(graphNode.x, graphNode.y, size + 4, 0, 2 * Math.PI);
          ctx.fillStyle = color;
          ctx.fill();
        }}
        linkColor={() => COLORS.borderSubtle}
        linkWidth={1.5}
        onNodeClick={handleNodeClick}
        backgroundColor={COLORS.deepEarth}
        cooldownTicks={100}
        d3AlphaDecay={0.02}
        d3VelocityDecay={0.3}
      />
    </div>
  );
}
