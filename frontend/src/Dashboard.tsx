import { useEffect, useMemo, useState } from 'react';
import { App, type McpUiHostContext, applyDocumentTheme, applyHostFonts, applyHostStyleVariables } from '@modelcontextprotocol/ext-apps';

import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Database, Zap, Activity } from 'lucide-react';
import { Badge } from '@/components/ui/badge';

export function Dashboard() {
    const [connected, setConnected] = useState(false);
    const app = useMemo(() => new App({ name: "Synapse Dashboard", version: "1.0.0" }), []);

    useEffect(() => {
        const handleHostContextChanged = (ctx: McpUiHostContext) => {
            if (ctx.theme) applyDocumentTheme(ctx.theme);
            if (ctx.styles?.variables) applyHostStyleVariables(ctx.styles.variables);
            if (ctx.styles?.css?.fonts) applyHostFonts(ctx.styles.css.fonts);
        };

        app.onhostcontextchanged = handleHostContextChanged;

        app.connect().then(() => {
             console.log("Connected to MCP Host");
             setConnected(true);
             const ctx = app.getHostContext();
             if (ctx) handleHostContextChanged(ctx);
        }).catch(err => {
             console.error("Failed to connect to host:", err);
        });

        return () => {
            app.onteardown = async () => { return {} };
        };
    }, [app]);


    return (
        <div className="min-h-screen bg-background text-foreground p-8">
            <div className="flex items-center justify-between mb-8">
                <div>
                    <h1 className="text-3xl font-bold tracking-tight">Synapse Memory Core</h1>
                    <p className="text-muted-foreground mt-2">Symbolic Engine Dashboard</p>
                </div>
                <Badge variant={connected ? "default" : "secondary"} className="text-sm px-3 py-1">
                    <Activity className={`w-4 h-4 mr-2 ${connected ? 'text-primary-foreground' : 'text-muted-foreground'}`} />
                    {connected ? 'Online - Connected to MCP' : 'Offline'}
                </Badge>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
                <Card>
                    <CardHeader className="flex flex-row items-center justify-between pb-2 space-y-0">
                        <CardTitle className="text-sm font-medium">Triples Indexed</CardTitle>
                        <Database className="w-4 h-4 text-muted-foreground" />
                    </CardHeader>
                    <CardContent>
                        <div className="text-2xl font-bold">4,092</div>
                        <p className="text-xs text-muted-foreground mt-1">
                            Deterministic Graph Triples
                        </p>
                    </CardContent>
                </Card>
                <Card>
                    <CardHeader className="flex flex-row items-center justify-between pb-2 space-y-0">
                        <CardTitle className="text-sm font-medium">Ontology Inferences</CardTitle>
                        <Zap className="w-4 h-4 text-muted-foreground" />
                    </CardHeader>
                    <CardContent>
                        <div className="text-2xl font-bold">12,450</div>
                        <p className="text-xs text-muted-foreground mt-1">
                            OWL-RL Derived Facts
                        </p>
                    </CardContent>
                </Card>
                <Card>
                    <CardHeader className="flex flex-row items-center justify-between pb-2 space-y-0">
                        <CardTitle className="text-sm font-medium">Active Namespaces</CardTitle>
                        <Activity className="w-4 h-4 text-muted-foreground" />
                    </CardHeader>
                    <CardContent>
                        <div className="flex flex-wrap gap-2 mt-1">
                            <Badge variant="secondary">default</Badge>
                            <Badge variant="secondary">research-agent</Badge>
                            <Badge variant="secondary">schema</Badge>
                        </div>
                    </CardContent>
                </Card>
            </div>
        </div>
    );
}