{{- define "slskr.name" -}}
slskr
{{- end -}}

{{- define "slskr.fullname" -}}
{{ .Release.Name }}-slskr
{{- end -}}
