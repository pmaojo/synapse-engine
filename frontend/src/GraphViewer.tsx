import { useEffect, useState, useMemo } from 'react';
import { useLocation } from 'react-router-dom';
import { App, type McpUiHostContext, applyDocumentTheme, applyHostFonts, applyHostStyleVariables } from '@modelcontextprotocol/ext-apps';
import type { CallToolResult } from '@modelcontextprotocol/sdk/types.js';

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Box, Layers, RefreshCcw } from 'lucide-react';
import { Button } from '@/components/ui/button';

export function GraphViewer() {
    const location = useLocation();
    const queryParams = new URLSearchParams(location.search);
    const entityUri = queryParams.get('uri') || '';

    const [graphData, setGraphData] = useState<any>(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    const app = useMemo(() => new App({ name: "Synapse Graph Viewer", version: "1.0.0" }), []);

    useEffect(() => {
        const handleHostContextChanged = (ctx: McpUiHostContext) => {
            if (ctx.theme) applyDocumentTheme(ctx.theme);
            if (ctx.styles?.variables) applyHostStyleVariables(ctx.styles.variables);
            if (ctx.styles?.css?.fonts) applyHostFonts(ctx.styles.css.fonts);
        };

        app.ontoolresult = (result: CallToolResult) => {
            console.log("Received tool result:", result);
            try {
                // We expect the Rust server to return the JSON representation of the graph
                // as part of the structuredContent or standard text content
                let data = null;
                if (result.structuredContent) {
                    data = result.structuredContent;
                } else if (result.content && result.content.length > 0) {
                     const textContent = result.content.find((c: any) => c.type === 'text');
                     if (textContent && 'text' in textContent) {
                         // The Rust code currently returns: `Neighborhood of {uri}:\n{json}`
                         const parts = textContent.text.split('\n');
                         if (parts.length >= 2) {
                             data = JSON.parse(parts.slice(1).join('\n'));
                         } else {
                             data = JSON.parse(textContent.text);
                         }
                     }
                }

                if (data) {
                     setGraphData(data);
                } else {
                     setError("No graph data found in the tool response.");
                }
            } catch (err: any) {
                setError(`Failed to parse graph data: ${err.message}`);
            } finally {
                setLoading(false);
            }
        };

        app.onerror = (err) => {
            console.error("MCP App Error:", err);
            setError(err.message || "Unknown error occurred");
            setLoading(false);
        };

        app.onhostcontextchanged = handleHostContextChanged;

        app.connect().then(() => {
             console.log("Connected to MCP Host");
             const ctx = app.getHostContext();
             if (ctx) handleHostContextChanged(ctx);
        }).catch(err => {
             console.error("Failed to connect to host:", err);
             setError("Failed to connect to MCP host. Make sure this is running inside an MCP client.");
             setLoading(false);
        });

        return () => {
            // Cleanup on unmount
            app.onteardown = async () => { return {} };
        };
    }, [app]);

    const handleRefresh = async () => {
        setLoading(true);
        setError(null);
        try {
            await app.callServerTool({ name: "get_entity_neighborhood", arguments: { uri: entityUri } });
        } catch (err: any) {
             setError(`Failed to call tool: ${err.message}`);
             setLoading(false);
        }
    };


    if (loading && !graphData) {
        return (
            <div className="flex flex-col items-center justify-center min-h-screen bg-background text-foreground p-4">
                 <RefreshCcw className="h-8 w-8 animate-spin text-muted-foreground mb-4" />
                 <p className="text-muted-foreground">Waiting for graph data from host...</p>
                 <p className="text-xs text-muted-foreground mt-2">Connecting to {entityUri}</p>
            </div>
        );
    }

    if (error) {
        return (
             <div className="flex flex-col items-center justify-center min-h-screen bg-background p-4">
                 <Card className="w-full max-w-md border-destructive/50">
                    <CardHeader>
                        <CardTitle className="text-destructive">Error Loading Graph</CardTitle>
                        <CardDescription>{entityUri}</CardDescription>
                    </CardHeader>
                    <CardContent>
                        <p className="text-sm text-muted-foreground break-words">{error}</p>
                        <Button variant="outline" className="mt-4 w-full" onClick={() => window.location.reload()}>Retry</Button>
                    </CardContent>
                 </Card>
            </div>
        )
    }

    // A simple visualization since D3 can be complex to set up perfectly without knowing exact schema
    return (
        <div className="min-h-screen bg-background text-foreground p-6">
            <div className="flex items-center justify-between mb-6 border-b pb-4">
                <div>
                    <h1 className="text-2xl font-bold flex items-center gap-2">
                        <Box className="h-6 w-6 text-primary" />
                        Entity Neighborhood
                    </h1>
                    <p className="text-muted-foreground font-mono text-sm mt-1 break-all">{entityUri}</p>
                </div>
                <Button variant="outline" size="sm" onClick={handleRefresh} disabled={loading}>
                    <RefreshCcw className={`h-4 w-4 mr-2 ${loading ? 'animate-spin' : ''}`} />
                    Refresh
                </Button>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                {graphData?.nodes?.map((node: any, idx: number) => (
                    <Card key={idx} className={node.id === entityUri ? "border-primary shadow-sm" : ""}>
                         <CardHeader className="pb-2">
                             <CardTitle className="text-sm font-medium break-all flex items-start justify-between gap-2">
                                <span>{node.id}</span>
                                {node.id === entityUri && <Badge variant="default">Target</Badge>}
                             </CardTitle>
                         </CardHeader>
                         <CardContent>
                              {node.labels && node.labels.length > 0 && (
                                  <div className="flex flex-wrap gap-1 mt-2">
                                      {node.labels.map((label: string, i: number) => (
                                          <Badge key={i} variant="secondary" className="text-xs">{label}</Badge>
                                      ))}
                                  </div>
                              )}
                         </CardContent>
                    </Card>
                ))}
            </div>

            {(!graphData?.nodes || graphData.nodes.length === 0) && (
                 <div className="text-center py-12 text-muted-foreground border border-dashed rounded-lg">
                     <Layers className="h-12 w-12 mx-auto mb-4 opacity-20" />
                     <p>No nodes found in the neighborhood.</p>
                 </div>
            )}
        </div>
    );
}
