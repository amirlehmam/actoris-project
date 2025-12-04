{{/*
Expand the name of the chart.
*/}}
{{- define "actoris.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
*/}}
{{- define "actoris.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "actoris.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "actoris.labels" -}}
helm.sh/chart: {{ include "actoris.chart" . }}
{{ include "actoris.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "actoris.selectorLabels" -}}
app.kubernetes.io/name: {{ include "actoris.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Create the name of the service account to use
*/}}
{{- define "actoris.serviceAccountName" -}}
{{- if .Values.serviceAccount.create }}
{{- default (include "actoris.fullname" .) .Values.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.serviceAccount.name }}
{{- end }}
{{- end }}

{{/*
IdentityCloud fullname
*/}}
{{- define "actoris.identityCloud.fullname" -}}
{{- printf "%s-identity-cloud" (include "actoris.fullname" .) | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
TrustLedger fullname
*/}}
{{- define "actoris.trustLedger.fullname" -}}
{{- printf "%s-trustledger" (include "actoris.fullname" .) | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
OneBill fullname
*/}}
{{- define "actoris.oneBill.fullname" -}}
{{- printf "%s-onebill" (include "actoris.fullname" .) | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Darwinian fullname
*/}}
{{- define "actoris.darwinian.fullname" -}}
{{- printf "%s-darwinian" (include "actoris.fullname" .) | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Sidecar fullname
*/}}
{{- define "actoris.sidecar.fullname" -}}
{{- printf "%s-sidecar" (include "actoris.fullname" .) | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Image pull secrets
*/}}
{{- define "actoris.imagePullSecrets" -}}
{{- with .Values.global.imagePullSecrets }}
imagePullSecrets:
{{- toYaml . | nindent 2 }}
{{- end }}
{{- end }}
