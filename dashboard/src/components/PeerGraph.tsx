import { useCallback, useRef, useEffect, useMemo } from 'react';
import ForceGraph2D, { ForceGraphMethods } from 'react-force-graph-2d';
import type { GraphNode, GraphLink } from '@/types';

interface PeerGraphProps {
  nodes: GraphNode[];
  links: GraphLink[];
  onNodeClick?: (nodeId: string) => void;
  selectedNodeId?: string | null;
}

export function PeerGraph({ nodes, links, onNodeClick, selectedNodeId }: PeerGraphProps) {
  const graphRef = useRef<ForceGraphMethods>();

  // Memoize graph data to prevent re-renders from creating new objects
  const graphDataMemo = useMemo(() => ({ nodes, links }), [nodes, links]);

  // Color scale based on reputation
  const getNodeColor = useCallback((node: GraphNode) => {
    if (node.isLocal) return '#60a5fa'; // blue for local
    const score = node.reputation;
    if (score >= 0.9) return '#22c55e'; // green - excellent
    if (score >= 0.7) return '#84cc16'; // lime - good
    if (score >= 0.5) return '#a3a3a3'; // gray - neutral
    if (score >= 0.3) return '#f59e0b'; // amber - poor
    return '#ef4444'; // red - untrusted
  }, []);

  const getNodeSize = useCallback((node: GraphNode) => {
    return node.isLocal ? 12 : 8 + node.reputation * 4;
  }, []);

  // Highlight selected node
  const nodeCanvasObject = useCallback(
    (node: any, ctx: CanvasRenderingContext2D, globalScale: number) => {
      if (node.x === undefined || node.y === undefined) return;
      const graphNode = node as GraphNode;

      const label = graphNode.name || String(graphNode.id).slice(0, 8);
      const fontSize = 12 / globalScale;
      const size = getNodeSize(graphNode);
      const color = getNodeColor(graphNode);
      const isSelected = graphNode.id === selectedNodeId;

      // Draw glow for selected node
      if (isSelected) {
        ctx.beginPath();
        ctx.arc(node.x, node.y, size + 4, 0, 2 * Math.PI);
        ctx.fillStyle = `${color}44`;
        ctx.fill();
      }

      // Draw node
      ctx.beginPath();
      ctx.arc(node.x, node.y, size, 0, 2 * Math.PI);
      ctx.fillStyle = color;
      ctx.fill();

      // Draw border
      ctx.strokeStyle = isSelected ? '#fff' : '#333';
      ctx.lineWidth = isSelected ? 2 / globalScale : 1 / globalScale;
      ctx.stroke();

      // Draw label
      ctx.font = `${fontSize}px Inter, sans-serif`;
      ctx.textAlign = 'center';
      ctx.textBaseline = 'top';
      ctx.fillStyle = '#fff';
      ctx.fillText(label, node.x, node.y + size + 2);
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
    <div className="w-full h-full bg-surface rounded-lg overflow-hidden">
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
        linkColor={() => '#4b5563'}
        linkWidth={1}
        onNodeClick={handleNodeClick}
        backgroundColor="#111816"
        cooldownTicks={100}
        d3AlphaDecay={0.02}
        d3VelocityDecay={0.3}
      />
    </div>
  );
}
